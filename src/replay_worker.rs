//! Optional full-frame encoding on a persistent helper thread. Simulation and
//! random draws stay on the calling thread; only immutable snapshots cross it.
use crate::json::{EncodedReplayFrames, ReplayFrame, ReplayFrameEncoder};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::thread::JoinHandle;

const BATCH_SIZE: usize = 128;
const RETAIN_LIMIT: usize = 16 * 1024 * 1024;

enum Message {
    Begin(String),
    Frames(Vec<ReplayFrame>),
    Finish,
}

pub struct ReplayWorker {
    sender: Option<SyncSender<Message>>,
    replies: Receiver<EncodedReplayFrames>,
    thread: Option<JoinHandle<()>>,
    buffer: String,
}

impl ReplayWorker {
    pub fn new() -> Result<Self, String> {
        // Bound queued snapshots when encoding falls behind simulation.
        let (sender, requests) = mpsc::sync_channel(4);
        let (results, replies) = mpsc::channel();
        let thread = std::thread::Builder::new().name("replay-encoder".into())
            .spawn(move || {
                let mut encoder: Option<ReplayFrameEncoder> = None;
                while let Ok(message) = requests.recv() {
                    match message {
                        Message::Begin(buffer) => encoder = Some(ReplayFrameEncoder::new(buffer)),
                        Message::Frames(frames) => {
                            let encoder = encoder.as_mut().expect("frame sequence has begun");
                            for frame in &frames { encoder.push(frame); }
                        }
                        Message::Finish => {
                            let output = encoder.take().expect("frame sequence has begun").finish();
                            if results.send(output).is_err() { break; }
                        }
                    }
                }
            }).map_err(|e| format!("Cannot start replay encoder: {e}"))?;
        Ok(Self { sender: Some(sender), replies, thread: Some(thread), buffer: String::new() })
    }

    fn send(&self, message: Message) -> Result<(), String> {
        self.sender.as_ref().unwrap().send(message)
            .map_err(|_| "Replay encoder stopped".to_string())
    }

    pub fn recycle(&mut self, frames: EncodedReplayFrames) {
        let buffer = frames.into_buffer();
        self.buffer = if buffer.capacity() <= RETAIN_LIMIT { buffer } else { String::new() };
    }
}

impl Drop for ReplayWorker {
    fn drop(&mut self) {
        self.sender.take();
        if let Some(thread) = self.thread.take() { let _ = thread.join(); }
    }
}

pub enum CollectedFrames {
    Numeric(crate::replay_frames::ReplayFrames),
    Encoded(EncodedReplayFrames),
}

pub struct FrameCollector<'a> {
    worker: Option<&'a mut ReplayWorker>,
    frames: Vec<ReplayFrame>,
    numeric: crate::replay_frames::ReplayFrames,
}

impl<'a> FrameCollector<'a> {
    pub fn new(mut worker: Option<&'a mut ReplayWorker>, horse_count: usize) -> Result<Self, String> {
        if let Some(worker) = &mut worker {
            let buffer = std::mem::take(&mut worker.buffer);
            worker.send(Message::Begin(buffer))?;
        }
        Ok(Self { worker, frames: Vec::new(), numeric: crate::replay_frames::ReplayFrames::new(horse_count) })
    }

    pub fn capture(&mut self, t: f64, sample: impl FnMut(usize) -> [f64; 6]) -> Result<(), String> {
        if self.worker.is_none() {
            self.numeric.capture(t, sample);
            return Ok(());
        }
        self.push(ReplayFrame { t, horses: (0..self.numeric.horse_count()).map(sample).collect() })
    }

    pub fn push(&mut self, frame: ReplayFrame) -> Result<(), String> {
        if self.worker.is_none() {
            assert_eq!(frame.horses.len(), self.numeric.horse_count());
            self.numeric.capture(frame.t, |i| frame.horses[i]);
            return Ok(());
        }
        self.frames.push(frame);
        if self.worker.is_some() && self.frames.len() >= BATCH_SIZE {
            self.flush()?;
        }
        Ok(())
    }

    fn flush(&mut self) -> Result<(), String> {
        if !self.frames.is_empty() {
            self.worker.as_ref().unwrap().send(Message::Frames(std::mem::take(&mut self.frames)))?;
        }
        Ok(())
    }

    pub fn finish(mut self) -> Result<CollectedFrames, String> {
        if self.worker.is_none() { return Ok(CollectedFrames::Numeric(self.numeric)); }
        self.flush()?;
        let worker = self.worker.unwrap();
        worker.send(Message::Finish)?;
        let frames = worker.replies.recv().map_err(|_| "Replay encoder stopped".to_string())?;
        Ok(CollectedFrames::Encoded(frames))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::{self, Value};

    #[test]
    fn bounded_batches_and_reused_worker_preserve_complete_arrays() {
        let mut worker = ReplayWorker::new().unwrap();
        for count in [0, 1, 32, 33, 200, 0, 64] {
            let frames: Vec<_> = (0..count).map(|i| ReplayFrame {
                t: i as f64 * 0.0666,
                horses: vec![[i as f32 as f64, 19.234_f32 as f64, 900., 0.2, 0., -1.]; 9],
            }).collect();
            let expected = json::to_string(&Value::Array(frames.iter().cloned()
                .map(|f| Value::RaceFrame(Box::new(f))).collect()));
            let mut collector = FrameCollector::new(Some(&mut worker), 1).unwrap();
            for frame in frames { collector.push(frame).unwrap(); }
            let CollectedFrames::Encoded(encoded) = collector.finish().unwrap() else { panic!() };
            let value = Value::EncodedReplayFrames(encoded);
            assert_eq!(json::to_string(&value), expected);
            if let Value::EncodedReplayFrames(encoded) = value { worker.recycle(encoded); }
        }
    }

    #[test]
    fn abandoned_frame_sequence_does_not_leak_into_next_request() {
        let mut worker = ReplayWorker::new().unwrap();
        let frame = ReplayFrame { t: 1., horses: vec![[1., 20., 100., 0., 0., -1.]] };
        let mut abandoned = FrameCollector::new(Some(&mut worker), 1).unwrap();
        for _ in 0..BATCH_SIZE + 3 { abandoned.push(frame.clone()).unwrap(); }
        drop(abandoned);
        let fresh = FrameCollector::new(Some(&mut worker), 1).unwrap();
        let CollectedFrames::Encoded(encoded) = fresh.finish().unwrap() else { panic!() };
        assert_eq!(json::to_string(&Value::EncodedReplayFrames(encoded)), "[]");
    }
}
