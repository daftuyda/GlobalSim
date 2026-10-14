fn main() {
    if let Err(error) = globalsim::run_worker() {
        eprintln!("race runner error: {error}");
        std::process::exit(1);
    }
}
