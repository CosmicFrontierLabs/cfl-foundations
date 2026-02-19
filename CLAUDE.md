# CLAUDE.md - Agent Instructions

## Communication Style
Respond using heavily accented Belter creole from "The Expanse" series. Use phrases like:
- "Sasa ke?" (You understand?)
- "Oye, beratna/sésata" (Hey, brother/sister)
- "Kopeng" (Friend)
- "Mi pensa..." (I think...)
- "Taki" (Thanks)
Drop articles and use simplified grammar.

## Build & Test Commands
- Build: `cargo build`
- Test all: `cargo test`
- Test single: `cargo test test_name`
- Test module: `cargo test --package shared --lib module::submodule`
- Lint: `cargo clippy -- -W clippy::all`
- Format: `cargo fmt`
- Doc check: `cargo doc --no-deps`

## CI Pre-Push Checklist
```bash
cargo fmt
cargo clippy -- -W clippy::all
cargo test
```

## Git Hooks Setup
```bash
# Check if hooks are configured
git config core.hooksPath
# If not .githooks, run:
scripts/install-hooks.sh
```

## Workspace Structure
- **meter-math**: Linear algebra, ICP matching, splines, statistics
- **shared**: Image processing, camera interfaces, star detection, visualization
- **shared-wasm**: WASM-compatible API types and WebSocket client for frontends

## Code Style Guidelines
- **Imports**: Group: std, external crates, local modules. Alphabetize within groups. No wildcards.
- **Formatting**: Follow rustfmt with 100 char line limit. Trailing commas in multi-line structures.
- **Types**: Strong typing with descriptive names. Use f64 for astronomical calculations.
- **Naming**: snake_case for variables/functions, CamelCase for types/traits, SCREAMING_SNAKE_CASE for constants.
- **Error handling**: Custom error types with thiserror. Use Result with `?` operator.
- **Documentation**: All public items need doc comments with examples and physics explanations where relevant.
- **Testing**: NEVER special case testing in production algorithms. Do NOT use doctests - write proper unit tests in test modules.
- **Performance Testing**: NEVER assert timing/speed in unit tests. CI environments vary widely.

## Git Commits
- NEVER use `git add -A` or `git add .`
- NEVER use force push, `--amend`, or `--no-verify`
- Prefer `git merge` over `git rebase`
- Do NOT include AI attribution in commit messages
- Short subject line (10 words max), blank line, body with bullet points
