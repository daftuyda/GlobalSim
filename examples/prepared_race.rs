//! Run with: cargo run --release --features fast-alloc --example prepared_race -- path/to/race.json 100003
use globalsim::PreparedRace;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    let input = std::fs::read_to_string(args.get(1).ok_or("Supply one JSON race request")?)?;
    let first_seed: i64 = args.get(2).map_or(Ok(100003), |s| s.parse())?;
    let mut race = PreparedRace::new(input.trim())?;
    for seed in first_seed..first_seed + 3 {
        let result = race.run(seed)?;
        println!("seed {seed}: {} frames, {} runners", result.frames().len(), result.frames().horse_count());
        // Borrow numeric values directly, or call result.write_json(...)
        // / result.into_binary(...) when a serialized reply is needed.
    }
    Ok(())
}
