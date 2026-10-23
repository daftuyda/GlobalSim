//! Worker support for sampling one race setup across many seeds.
//!
//! A setup is a request without its `seed`. Repeating a setup reuses one
//! prepared race, so each further seed pays only for its own simulation.
//! Every reply is byte-identical to the reply for that request sent alone;
//! anything the prepared path cannot run goes through the per-request path.
use crate::*;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};

/// Worker-only sampling controls, removed from a request before it is read as
/// a race. A request without them is answered exactly as before.
pub(crate) struct Sampling {
    /// One reply per entry, in order, each as if sent alone with that `seed`.
    pub seeds: Option<Vec<Value>>,
    pub threads: usize,
    pub capture_frames: bool,
}

impl Sampling {
    pub fn take(request: &mut Value) -> Result<Self, String> {
        let mut sampling = Sampling { seeds: None, threads: 1, capture_frames: true };
        let Value::Object(map) = request else { return Ok(sampling) };
        match map.remove("seeds") {
            None => {}
            Some(Value::Array(seeds)) if !seeds.is_empty() => sampling.seeds = Some(seeds),
            Some(_) => return Err("seeds must be a non-empty array".into()),
        }
        if let Some(value) = map.remove("threads") {
            sampling.threads = match value.as_f64() {
                Some(0.) => std::thread::available_parallelism().map_or(1, |n| n.get()),
                Some(n) if n.fract() == 0. && (1. ..=256.).contains(&n) => n as usize,
                _ => return Err("threads must be an integer from 0 to 256".into()),
            };
        }
        if let Some(value) = map.remove("omitFrames") {
            sampling.capture_frames = !value.as_bool().ok_or("omitFrames must be a boolean")?;
        }
        Ok(sampling)
    }
}

/// `request` with `seed` replaced, as a client would send it alone.
pub(crate) fn with_seed(request: &Value, seed: &Value) -> Value {
    let mut request = request.clone();
    if let Value::Object(map) = &mut request { map.insert("seed".into(), seed.clone()); }
    request
}

/// Two requests are the same setup when every key but `seed` is bitwise equal.
fn same_setup(a: &Value, b: &Value) -> bool {
    let (Value::Object(a), Value::Object(b)) = (a, b) else { return false };
    let mut a = a.iter().filter(|(key, _)| key.as_str() != "seed");
    let mut b = b.iter().filter(|(key, _)| key.as_str() != "seed");
    loop {
        match (a.next(), b.next()) {
            (None, None) => return true,
            (Some((ka, va)), Some((kb, vb))) if ka == kb && va.bitwise_eq(vb) => {}
            _ => return false,
        }
    }
}

enum Slot {
    Seen,
    Ready(Box<PreparedRace>),
    Failed,
}

/// The most recent setup, prepared once it is needed more than once.
#[derive(Default)]
pub(crate) struct SetupCache {
    skills: Option<(Value, Rc<SkillTable>)>,
    setup: Option<(Value, Slot)>,
}

impl SetupCache {
    /// The prepared race for `request`'s setup. `None` sends the request down
    /// the per-request path: a setup's first lone request (`eager` false) skips
    /// preparation, and a setup that fails to prepare reports its own error.
    fn race(&mut self, request: &Value, eager: bool) -> Option<&mut PreparedRace> {
        if !self.setup.as_ref().is_some_and(|(setup, _)| same_setup(setup, request)) {
            self.setup = Some((request.clone(), Slot::Seen));
            if !eager { return None; }
        }
        if matches!(self.setup, Some((_, Slot::Seen))) {
            let slot = match self.prepare(request) { Some(race) => Slot::Ready(Box::new(race)), None => Slot::Failed };
            self.setup.as_mut().unwrap().1 = slot;
        }
        match &mut self.setup {
            Some((_, Slot::Ready(race))) => Some(race),
            _ => None,
        }
    }

    fn prepare(&mut self, request: &Value) -> Option<PreparedRace> {
        let raw = match request.get("skillData") {
            Some(data) => data,
            None => bundled_skills().ok()?,
        };
        let skills = match &self.skills {
            Some((data, table)) if data.bitwise_eq(raw) => table.clone(),
            _ => {
                let table = Rc::new(load_skill_value(raw).ok()?);
                self.skills = Some((raw.clone(), table.clone()));
                table
            }
        };
        PreparedRace::prepare(request.clone(), Some(skills)).ok()
    }

    /// The prepared reply for `request`, or `None` for the per-request path.
    /// Run errors also take that path, so error text never depends on caching.
    pub fn reply(&mut self, request: &Value, eager: bool, capture_frames: bool) -> Option<Value> {
        let seed = request.get("seed").and_then(Value::as_i64)?;
        self.race(request, eager)?.run_value(seed, capture_frames).ok()
    }
}

/// One reply as the worker writes it: a JSON line or a binary packet.
fn encode(reply: &mut Value, binary: bool, metadata: &mut String) -> Vec<u8> {
    let mut packet = Vec::new();
    if binary {
        binary_replay::encode(reply, metadata, &mut packet);
    } else {
        json::to_string_into(reply, metadata);
        metadata.push('\n');
        packet.extend_from_slice(metadata.as_bytes());
    }
    packet
}

struct Job {
    request: Value,
    seeds: Vec<Value>,
    next: AtomicUsize,
    capture_frames: bool,
    binary: bool,
}

/// A seed's encoded reply, or `None` when the caller must answer it alone.
type Reply = (usize, Option<Vec<u8>>);

/// Persistent sampling threads. Each keeps its own prepared setup, because a
/// prepared race is confined to the thread that built it.
#[derive(Default)]
pub(crate) struct SeedPool {
    threads: Vec<mpsc::Sender<(Arc<Job>, mpsc::Sender<Reply>)>>,
}

/// What the pool hands back, in seed order.
pub(crate) enum Event {
    Reply(usize, Option<Vec<u8>>),
    /// No further reply is ready yet: a good moment to flush.
    Idle,
}

impl SeedPool {
    /// Run `seeds` for one setup on up to `threads` threads, handing replies
    /// back in seed order as soon as each is next.
    pub fn run(&mut self, request: Value, seeds: Vec<Value>, threads: usize, capture_frames: bool, binary: bool,
        mut emit: impl FnMut(Event) -> Result<(), String>) -> Result<(), String> {
        let threads = threads.min(seeds.len());
        while self.threads.len() < threads {
            let (sender, jobs) = mpsc::channel();
            std::thread::Builder::new().name("globalsim-seeds".into())
                .spawn(move || serve(jobs)).map_err(|e| e.to_string())?;
            self.threads.push(sender);
        }
        let count = seeds.len();
        let job = Arc::new(Job { request, seeds, next: AtomicUsize::new(0), capture_frames, binary });
        let (sender, replies) = mpsc::channel();
        for thread in &self.threads[..threads] {
            thread.send((job.clone(), sender.clone())).map_err(|_| "Seed thread stopped".to_string())?;
        }
        drop(sender);
        let mut waiting = BTreeMap::new();
        let (mut next, mut unflushed) = (0, false);
        while next < count {
            let (index, reply) = match replies.try_recv() {
                Ok(reply) => reply,
                Err(mpsc::TryRecvError::Empty) => {
                    // Flush before blocking, so a reader sees each reply promptly.
                    if unflushed { emit(Event::Idle)?; unflushed = false; }
                    replies.recv().map_err(|_| "Seed thread stopped".to_string())?
                }
                Err(mpsc::TryRecvError::Disconnected) => return Err("Seed thread stopped".into()),
            };
            waiting.insert(index, reply);
            while let Some(reply) = waiting.remove(&next) {
                emit(Event::Reply(next, reply))?;
                (next, unflushed) = (next + 1, true);
            }
        }
        if unflushed { emit(Event::Idle)?; }
        Ok(())
    }
}

fn serve(jobs: mpsc::Receiver<(Arc<Job>, mpsc::Sender<Reply>)>) {
    let mut setups = SetupCache::default();
    let mut metadata = String::new();
    for (job, replies) in jobs {
        loop {
            let index = job.next.fetch_add(1, Ordering::Relaxed);
            let Some(seed) = job.seeds.get(index) else { break };
            let reply = setups.reply(&with_seed(&job.request, seed), true, job.capture_frames)
                .map(|mut reply| encode(&mut reply, job.binary, &mut metadata));
            if replies.send((index, reply)).is_err() { break; }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stateful_preconditions_are_fresh_on_every_prepared_run() {
        let input = include_str!("../tests/fixtures/stateful_precondition_reuse.json");
        let request = json::parse(input).unwrap();
        let seed = request.get("seed").unwrap().as_i64().unwrap();
        let mut setups = SetupCache::default();
        for seed in [seed, seed, seed ^ 7919, seed] {
            let at = with_seed(&request, &json::num(seed as f64));
            let mut fresh = PreparedRace::new(&json::to_string(&at)).unwrap();
            let mut expected = fresh.run_value(seed, true).unwrap();
            assert_eq!(expected.get_array("skillEvents").unwrap().len(), 1);
            let mut actual = setups.reply(&at, true, true).unwrap();
            assert!(encode(&mut actual, true, &mut String::new())
                == encode(&mut expected, true, &mut String::new()),
                "Stateful precondition leaked between prepared runs at seed {seed}");
        }
    }

    #[test]
    fn stateful_preconditions_are_fresh_in_reused_seed_threads() {
        let input = include_str!("../tests/fixtures/stateful_precondition_reuse.json");
        let request = json::parse(input).unwrap();
        let seed = request.get("seed").unwrap().as_i64().unwrap();
        let seeds: Vec<_> = [seed, seed, seed ^ 7919, seed].into_iter()
            .map(|s| json::num(s as f64)).collect();
        let expected: Vec<_> = seeds.iter().map(|seed| {
            let mut race = PreparedRace::new(&json::to_string(&with_seed(&request, seed))).unwrap();
            let mut value = race.run_value(seed.as_i64().unwrap(), true).unwrap();
            assert_eq!(value.get_array("skillEvents").unwrap().len(), 1);
            encode(&mut value, true, &mut String::new())
        }).collect();
        let mut pool = SeedPool::default();
        for threads in [1, 2, 2, 1] {
            let mut replies = 0;
            pool.run(request.clone(), seeds.clone(), threads, true, true, |event| {
                if let Event::Reply(index, reply) = event {
                    assert!(reply.as_ref() == Some(&expected[index]),
                        "Stateful precondition leaked in a seed thread at reply {index}");
                    replies += 1;
                }
                Ok(())
            }).unwrap();
            assert_eq!(replies, seeds.len());
        }
    }

    #[test]
    fn sampling_controls_are_removed_and_validated() {
        let mut request = json::parse(r#"{"seed":1,"seeds":[3,4],"threads":2,"omitFrames":true,"x":0}"#).unwrap();
        let sampling = Sampling::take(&mut request).unwrap();
        assert_eq!(sampling.seeds.map(|seeds| seeds.len()), Some(2));
        assert_eq!((sampling.threads, sampling.capture_frames), (2, false));
        assert!(request.bitwise_eq(&json::parse(r#"{"seed":1,"x":0}"#).unwrap()));
        let plain = Sampling::take(&mut request).unwrap();
        assert!(plain.seeds.is_none() && plain.threads == 1 && plain.capture_frames);
        for raw in [r#"{"seeds":[]}"#, r#"{"seeds":5}"#, r#"{"threads":-1}"#, r#"{"threads":1.5}"#,
                    r#"{"threads":257}"#, r#"{"omitFrames":1}"#] {
            assert!(Sampling::take(&mut json::parse(raw).unwrap()).is_err(), "{raw}");
        }
    }

    #[test]
    fn repeated_setups_reply_exactly_as_fresh_preparations() {
        let line = include_str!("../examples/race.jsonl").trim();
        PreparedRace::new(line).unwrap();
        for retail in [false, true] {
            let mut request = json::parse(line).unwrap();
            if let Value::Object(map) = &mut request {
                map.insert("experimentalRetailRuntime".into(), Value::Bool(retail));
            }
            let fresh = |seed: f64, capture: bool| {
                let request = with_seed(&request, &json::num(seed));
                let mut race = PreparedRace::prepare(request, None).unwrap();
                json::to_string(&race.run_value(seed as i64, capture).unwrap())
            };
            let mut setups = SetupCache::default();
            let at = |seed: f64| with_seed(&request, &json::num(seed));
            // A setup's first lone request is left to the per-request path.
            assert!(setups.reply(&at(1.), false, true).is_none());
            for seed in [2., 3., 2.] {
                let reply = setups.reply(&at(seed), false, true).unwrap();
                assert_eq!(json::to_string(&reply), fresh(seed, true));
            }
            let reply = setups.reply(&at(4.), false, false).unwrap();
            assert_eq!(json::to_string(&reply), fresh(4., false));
            assert!(reply.get("frames").is_none() && reply.get("finishFrames").is_none());
            // A changed setup starts over; a batch prepares at once.
            let mut other = at(5.);
            if let Value::Object(map) = &mut other { map.insert("weather".into(), Value::String("Rainy".into())); }
            assert!(setups.reply(&other, false, true).is_none());
            assert!(setups.reply(&at(5.), true, true).is_some());
        }
    }
}
