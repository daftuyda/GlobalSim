use std::{hint::black_box, time::Instant};

fn main() {
    let args: Vec<_> = std::env::args().collect();
    let input = std::fs::read_to_string(&args[1]).unwrap();
    let repeats: usize = args[2].parse().unwrap();
    let mode = &args[3];
    let start = Instant::now();
    let mut race = globalsim::PreparedRace::new(input.trim()).unwrap();
    let preparation = start.elapsed().as_secs_f64() * 1000.;
    let mut times = Vec::new();
    let mut metadata = String::new();
    let mut packet = Vec::new();
    let mut frames = 0;
    for index in 0..5 + 30 * repeats {
        let seed = 100003 + (index % 30) as i64 * 7919;
        let start = Instant::now();
        let result = race.run(seed).unwrap();
        frames += result.frames().len();
        if mode == "binary" {
            result.into_binary(&mut metadata, &mut packet);
            black_box(&packet);
        } else {
            black_box(result.frames());
            drop(result);
        }
        if index >= 5 { times.push(start.elapsed().as_secs_f64() * 1000.); }
    }
    times.sort_by(f64::total_cmp);
    println!("{{\"mode\":\"{mode}\",\"runs\":{},\"preparation_ms\":{preparation},\"mean_ms\":{},\"median_ms\":{},\"p95_ms\":{},\"total_frames\":{frames}}}",
        times.len(), times.iter().sum::<f64>() / times.len() as f64,
        times[times.len()/2], times[times.len()*95/100]);
}
