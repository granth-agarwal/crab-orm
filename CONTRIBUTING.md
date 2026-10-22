# Contributing

This repository implements foundations, owned query types, and Phase 1.3 SQLite
compilation/execution. Start with the
[scope](docs/product-scope.md), [architecture](docs/architecture.md), and ADRs linked
there. Discuss changes to public contracts alongside their limitations; do not
add placeholders that panic or return invented success.

## Local setup and validation

Install [rustup](https://rustup.rs/) and Git, then run commands from the repository
root. Rustup reads `rust-toolchain.toml` and installs stable, rustfmt, and Clippy.
The current edition is 2024, resolver 3; no MSRV policy has been verified.
Install a C toolchain for bundled SQLite. Tests use private in-memory connections
and temporary directories; no database service, environment variables, or credentials
are required. Never mutate process environment variables in tests.

```sh
rustup show active-toolchain
cargo check --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo build --workspace --all-features --locked
cargo build --workspace --exclude sqlite-todo --no-default-features --locked
cargo test --workspace --exclude sqlite-todo --no-default-features --locked
cargo tree -p crab-driver -p crab-orm --no-default-features --edges normal --locked
cargo run -p sqlite-todo --locked
git diff --check
```

Use `cargo fmt --all` to apply formatting. Add focused tests for actual behavior;
SQL generation changes must update SQL/bind goldens and cover non-interpolation,
grouping, and pagination. Driver changes must cover isolated SQLite round trips,
constraints, bind counts, and runtime storage classes; facade integration tests cover all four compiled
statement types. Compile-fail doctests guard private fields and consuming close.
Transaction cancellation/conformance and model mapping remain future work; these
tests do not establish ORM correctness. Exclude sqlite-todo from no-default checks:
its explicit sqlite dependency would otherwise enable SQLx through feature unification.
The normal library dependency graph must contain no SQLx when sqlite is disabled.
Review documentation against the behavior actually implemented before committing.

## Crate and dependency policy

Keep internal dependencies acyclic: `crab-core` has no internal dependencies;
`crab-query` and `crab-driver` each depend internally only on it; `crab-orm` depends on those three;
the TODO example uses `crab-orm` plus Tokio. Model metadata and model mapping will
belong to `crab-orm`; database decoding belongs to the driver adapter.
Use owned CrabORM types at public boundaries. SeaQuery types stay private to the
SQLite compiler adapter; SQLx types stay private to the optional SQLite driver adapter.
Never add speculative crates or unused adapters.

Add dependencies only for implemented needs, verify current official APIs, review
licenses/security advisories, and select minimal features. SeaQuery 1.0.2 is used
only by `crab-query`, with default features disabled and `backend-sqlite` enabled.
SQLx 0.9.0 is optional only in crab-driver, default features off, with
sqlite-bundled and runtime-tokio. Do not enable macros, TLS, or unused backends.
Tokio drives the example and integration tests; tempfile isolates file tests.
SQLx execution uses runtime parameter binding, without a compile-time database
requirement. Keep driver/facade default features empty and test both modes. Shared
dependencies and lints belong in the root workspace manifest and must be inherited by member packages.

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
