# Contributing

This repository currently implements Phase 0 only. Start with the
[scope](docs/product-scope.md), [architecture](docs/architecture.md), and ADRs linked
there. Discuss changes to public contracts alongside their limitations; do not
add placeholders that panic or return invented success.

## Local setup and validation

Install [rustup](https://rustup.rs/) and Git, then run commands from the repository
root. Rustup reads `rust-toolchain.toml` and installs stable, rustfmt, and Clippy.
The current edition is 2024, resolver 3; no MSRV policy has been verified.
The contracts and example require no database, environment variables, or credentials.

```sh
rustup show active-toolchain
cargo check --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
cargo build --workspace --locked
cargo run -p sqlite-todo --locked
git diff --check
```

Use `cargo fmt --all` to apply formatting. Add focused tests for actual behavior;
SQL generation, row mapping, and transactions will need integration/conformance
tests when they exist. The current unit tests verify redaction, not ORM correctness.
Review documentation against the behavior actually implemented before committing.

## Crate and dependency policy

Keep the dependency graph acyclic: `crab-core` has no internal dependencies;
`crab-query` and `crab-driver` depend only on it; `crab-orm` depends on those three;
the TODO example depends only on `crab-orm`. Model metadata and model mapping will
belong to `crab-orm`; database decoding belongs to its future driver adapter.
Use owned CrabORM types at public boundaries. SQLx and SeaQuery types must remain
private to adapters once implemented. Never add speculative crates or unused adapters.

Add dependencies only for implemented needs, verify current official APIs, review
licenses/security advisories, and select minimal features. Phase 0 uses only `std`.
Tokio, SeaQuery, SQLx, and instrumentation dependencies remain deferred until used.
Future SQLx execution must use runtime parameter binding and must not introduce
a compile-time database requirement. Shared dependencies and lints belong in the
root workspace manifest and must be inherited by member packages.

Generate `Cargo.lock` using Cargo (for deliberate dependency updates, use
`cargo generate-lockfile` or `cargo update`), inspect the changes, and commit it.
Never hand-edit it. All packages remain `publish = false`; publishing and name
availability checks require a separate release task. Do not add an untested MSRV.

## Security, review, and commits

Bind values at runtime; never compose SQL from user values. Future dialect code
must validate/quote identifiers separately. Keep credentials, SQL containing
sensitive literals, parameter values, database files, and build artifacts out of
logs and commits. Public fields allow deliberate inspection, not blanket safe logging.

Keep changes scoped, preserve unrelated work, and review the full staged diff.
Use conventional commits, such as `fix: redact query diagnostics` or
`docs: clarify SQLite boolean mapping`. Describe actual behavior, tests, and known
limitations in reviews. Architecture changes should update the relevant ADR.
No performance or competitor superiority claim is acceptable without reproducible
evidence and a documented methodology.
