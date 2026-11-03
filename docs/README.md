# GlobalSim documentation

Start with the [project README](../README.md) for setup and an example race.
These guides describe v77 and the configuration used for its published
validation results.

## Race mechanics

The complete reference is available in [Markdown](RACE-MECHANICS.md) and
[Word](GlobalSim-Race-Mechanics-v77.docx). It covers runner attributes, course
physics, movement, stamina, skills and simulation order.

| Reference | Contents |
|---|---|
| [Mathematics](MATHEMATICS.md) | Stats, speed, acceleration, stamina, spurt and precision |
| [Field mechanics](FIELD-MECHANICS.md) | Pacing, passing, blocking, ranking and competition |
| [Skills](SKILLS.md) | Conditions, timing, effects, recipients and cooldowns |
| [Parameters](PARAMETERS.md) | Runtime parameters and selected source constants |

## Integration

| Guide | Contents |
|---|---|
| [Interface](INTERFACE.md) | JSON requests, seeds and result fields |
| [In-process API](IN-PROCESS.md) | Prepared Rust, C and Python integration |
| [Binary replay](BINARY-REPLAY.md) | Lossless binary replay protocol |
| [Architecture](ARCHITECTURE.md) | Data preparation, execution order and independent seed state |
| [Source index](SOURCE-INDEX.md) | Production modules, tools and data |

## Validation and performance

| Guide | Contents |
|---|---|
| [Accuracy](ACCURACY.md) | Observed agreement, comparison scope and limitations |
| [Testing](TESTING.md) | Build, regression and integration checks |
| [Performance](PERFORMANCE.md) | Measured latency and local benchmarking |
| [Optional comparison features](EXPERIMENTAL.md) | Cargo feature selection and configuration scope |

## Building the Word reference

The Word edition is generated from `RACE-MECHANICS.md`. Install Pandoc and
`python-docx`, then run:

```sh
python tools/export_mechanics_document.py
```

Review the rendered document after changing its source or exporter.
