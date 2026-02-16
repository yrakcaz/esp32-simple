# AI Coding Instructions

This document defines coding standards and maintenance practices for the ESP-Layground project.
Update this file when project practices change or guidelines become outdated.

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

## Maintenance & Synchronization

### Version Management
- When updating `rust-version` in Cargo.toml, also update the toolchain version in `.github/workflows/rust_ci.yml`
- The CI version should match the MSRV (Minimum Supported Rust Version)
- Verify the update by running CI workflow locally or checking CI results

### README.md Synchronization

#### Module List
- When adding/removing modules in `src/lib.rs`, update the "Library Modules" section in README.md
- Ensure module descriptions in README match the `///` doc comments in lib.rs
- Module count and names must be identical between both files

#### Example Documentation
- README examples (`cargo run --example client/server`) must reference actual example names from the examples/ directory
- Environment variable documentation must match what's checked in example code via `option_env!()`

### Library Documentation
- Module doc comments in `src/lib.rs` must be single-line descriptions synchronized with README.md
- Public functions/methods must document: `# Arguments`, `# Returns`, and `# Errors` sections
- Public structs/enums must have doc comments describing their purpose
- Public fields must have inline doc comments explaining their role
- Follow the existing documentation style

### Dependency Management
- Review exact version pins (`=x.y.z`) periodically to see if constraints can be relaxed
- Upgrading the ESP-IDF ecosystem (esp-idf-svc, esp-idf-hal, esp32-nimble) must be done together due to interdependencies
- ESP_IDF_VERSION in `.cargo/config.toml` should match the version expected by esp-idf dependencies

### CI/CD Workflow
- The CI workflow tests format, build, and clippy for both `client` and `server` binaries
- When adding new binaries to Cargo.toml, add corresponding CI matrix entries
- CI runs `cargo clippy -- -D warnings` (warnings denied) — all clippy suggestions must be fixed

### ESP-IDF Configuration
- When adding threads or features, verify stack sizes in `sdkconfig.defaults` are sufficient
- Ensure ESP_IDF_VERSION in `.cargo/config.toml` matches dependency requirements

### Feature Flags
- When adding new feature flags, document them in both README.md "Features" section and Cargo.toml
- Ensure CI tests relevant feature combinations if features affect compilation
