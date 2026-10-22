# CrabORM

A modular SQL ORM project with a Rust core, starting with Rust and SQLite.
**Current status: Phase 0 foundations, not a usable ORM.** No database is opened
by the current code, and no production readiness or performance advantage is claimed.

## Implemented

- Owned `Value`, distinguishable `OrmError` failures, and `CompiledQuery` containing
  SQL plus ordered parameters. Default query/error formatting redacts contents.
- A limited owned query input and `QueryCompiler` interface, without a renderer.
- Async `DatabaseExecutor` execute/fetch contracts and owned result types, without
  an executor implementation.
- An intentional `crab-orm` facade and a runnable contract inspection example.

Planned for Phase 1: model metadata/mapping, SQLite CRUD, transactions, a SeaQuery
compiler adapter, a SQLx runtime execution adapter, and a raw SQL escape hatch.
Tokio is the selected future runtime. None of these libraries is needed by the
current contracts, so there are no third-party Cargo dependencies yet.
PostgreSQL, MySQL, Python, migrations, sessions, and benchmarks come later.
SQL inspection, transactions, portability, and modularity are established ideas;
their usefulness together here remains a product hypothesis.

## Setup and checks

Install Rust through [rustup](https://rustup.rs/) and have Git available.
`rust-toolchain.toml` selects stable with rustfmt and Clippy; the workspace uses
edition 2024 and resolver 3. The initial foundation was validated with Rust 1.99.1
and Cargo 1.99.1 on macOS aarch64. No minimum supported Rust version (MSRV) has
been established. Stable intentionally follows rustup's stable channel; upgrades
must pass the checks. Initial toolchain installation needs network access.
No SQLite installation, database service, credentials, or SaaS account is needed.

From the repository root:

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

The example manually constructs parameterized SQL through `crab-orm`, prints its
SQL and bind count, and exits successfully. It neither compiles the SQL nor checks
its validity, connects to SQLite, executes it, or performs CRUD. Bind values are
not printed. See the [example guide](examples/sqlite-todo/README.md).

## Workspace

| Path | Responsibility |
| --- | --- |
| `crates/crab-core` | Owned values, compiled queries, errors |
| `crates/crab-query` | Limited input and replaceable compiler contract |
| `crates/crab-driver` | Async executor contract and owned results |
| `crates/crab-orm` | Public facade; future model metadata/mapping |
| `examples/sqlite-todo` | Phase 0 contract inspection binary |
| `docs` | Scope research, architecture, portability, and decisions |

All packages are unpublished (`publish = false`). Cargo generates the committed
workspace `Cargo.lock`. Tests currently cover formatting redaction and explicit
query inspection; their success is not evidence of ORM or database correctness.
CI runs the local gate commands on push and pull requests without a database.

## Design and contributing

- [Product scope, comparisons, and future validation hypotheses](docs/product-scope.md)
- [Architecture and implementation status](docs/architecture.md)
- [Portability and SQLite value policy](docs/portability.md)
- ADRs: [adapter boundaries](docs/decisions/0001-adapter-boundaries.md),
  [async-first API](docs/decisions/0002-async-first-api.md), and
  [raw SQL and portability](docs/decisions/0003-raw-sql-and-portability.md)
- [Contribution guide](CONTRIBUTING.md)

Licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
