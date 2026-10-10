# Testing

Run the Rust test suite with:

```sh
cargo test --release --locked
```

The most recent recorded run has **69 passing tests and 11 failures**. Failures
cover numerical reference comparisons, geometry, stamina and spurt behavior,
lateral movement, and duel conditions. See the [test output](../evidence/cargo-test.txt)
for individual assertions.

The release build and included example requests run successfully. Test results
and [official-race benchmark results](ACCURACY.md) measure different aspects
of the engine and should be considered together.
