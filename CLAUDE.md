# CLAUDE.md - Agent Instructions

## Build & Test Commands
- Build: `cargo build`
- Run: `cargo run --release`
- Test all: `cargo test`
- Test single: `cargo test test_name`
- Test module: `cargo test --package meter-sim --lib module::submodule`
- Lint: `cargo clippy -- -W clippy::all`
- Format: `cargo fmt`
- Benchmark: `cargo bench`

## Code Style Guidelines
- **Imports**: Group in order: std, external crates, local modules. Alphabetize within groups.
- **Formatting**: Follow rustfmt with 100 char line limit. Use trailing commas in multi-line structures.
- **Types**: Strong typing with descriptive names. Use f64 for astronomical calculations.
- **Naming**: snake_case for variables/functions, CamelCase for types/traits, SCREAMING_SNAKE_CASE for constants.
- **Error handling**: Custom error types with thiserror. Use Result with `?` operator.
- **Documentation**: All public items need doc comments with examples and physics explanations where relevant.
- **Architecture**: Separation between celestial mechanics, optics simulation, and tracking algorithms.
- **Performance**: Prefer vectorized operations. Profile computation-heavy code. Consider GPU acceleration for image processing.