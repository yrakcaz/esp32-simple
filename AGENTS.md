# AI Coding Instructions

## Code Style

### Functional over Imperative
- Prefer functional style over imperative
- Avoid using `return` statements — use expression-based returns instead
- Use `match`, `if let`, `map`, `and_then`, `unwrap_or_else` over early returns
- Prefer iterator methods (`map`, `filter`, `fold`) over `for` loops with mutation

### Error Handling
- Use `anyhow::Result` for error handling
- Use `?` operator — avoid `.unwrap()` except in tests
- Prefer `ok_or_else` / `map_err` over `match` for Option/Result conversions

### Logging
- Use `debug!` for function entry tracing (avoids ESP-IDF C-side verbose spam that `trace!` triggers)
- Use `info!` for business events
- Use `warn!` / `error!` for problems

### Formatting & Linting
- Run `cargo fmt` after every code change
- Run `cargo clippy` and fix warnings
- Follow standard Rust formatting conventions
