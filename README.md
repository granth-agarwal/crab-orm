# CrabORM

A modular SQL ORM project with a Rust core, starting with Rust and SQLite.
**Current status: Phase 1.2 SQLite compilation, not a usable ORM.** No database is opened
by the current code, and no production readiness or performance advantage is claimed.

## Implemented

- Owned `Value`, distinguishable `OrmError` failures, and `CompiledQuery` containing
  SQL plus ordered parameters. Default query/error formatting redacts contents.
- Validated table/column identifiers, grouped comparison/AND/OR predicates, and
  owned SELECT/INSERT/UPDATE/DELETE representations in `crab-core`.
- SELECT projections, ordering, and limit/offset options; paired, validated
  mutation assignments and explicit full-table mutation intent.
- A `QueryCompiler` interface and `SqliteCompiler` using a private SeaQuery adapter
  to produce SQLite SQL with placeholders and ordered owned bind values.
- Async `DatabaseExecutor` execute/fetch contracts and owned result types, without
  an executor implementation.
- An intentional `crab-orm` facade and a runnable SQL compilation example.

Remaining in Phase 1: model metadata/mapping, SQLite CRUD, transactions, a SQLx
runtime execution adapter, and a raw SQL escape hatch. Tokio is the selected
future runtime. Only `crab-query` depends on SeaQuery (1.0.2, SQLite backend only);
no database driver or runtime dependency is added yet.
PostgreSQL, MySQL, Python, migrations, sessions, and benchmarks come later.
SQL inspection, transactions, portability, and modularity are established ideas;
their usefulness together here remains a product hypothesis.

## Setup and checks

Install Rust through [rustup](https://rustup.rs/) and have Git available.
`rust-toolchain.toml` selects stable with rustfmt and Clippy; the workspace uses
edition 2024 and resolver 3. The initial foundation was validated with Rust 1.99.1
and Cargo 1.99.1 on macOS aarch64. No minimum supported Rust version (MSRV) has
been established. Stable intentionally follows rustup's stable channel; upgrades
must pass the checks. Initial toolchain/dependency downloads need network access;
tests and the example need none once dependencies are cached.
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

The example constructs four queries through `crab-orm`, compiles them with
`SqliteCompiler`, and prints generated SQL plus parameter counts. Bind values
are not printed; no database is contacted. See the
[example guide](examples/sqlite-todo/README.md),
[query construction reference](docs/query-types.md), and
[SQLite compiler usage and limitations](docs/sqlite-compiler.md).

## Workspace

| Path | Responsibility |
| --- | --- |
| `crates/crab-core` | Owned query/domain types, values, compiled queries, errors |
| `crates/crab-query` | Compiler contract and private SQLite SeaQuery adapter |
| `crates/crab-driver` | Async executor contract and owned results |
| `crates/crab-orm` | Public facade; future model metadata/mapping |
| `examples/sqlite-todo` | Query compilation binary, without execution |
| `docs` | Scope research, architecture, portability, and decisions |

All packages are unpublished (`publish = false`). Cargo generates the committed
workspace `Cargo.lock`. Tests cover construction invariants, grouping, ordering,
redaction, private construction boundaries, and SQLite SQL/bind goldens. They
verify compilation output, not ORM persistence or actual database correctness.
CI runs the local gate commands on push and pull requests without a database.

## Design and contributing

- [Product scope, comparisons, and future validation hypotheses](docs/product-scope.md)
- [Architecture and implementation status](docs/architecture.md)
- [Query types, constructors, and validation rules](docs/query-types.md)
- [SQLite compiler, parameter order, and limitations](docs/sqlite-compiler.md)
- [Portability and SQLite value policy](docs/portability.md)
- ADRs: [adapter boundaries](docs/decisions/0001-adapter-boundaries.md),
  [async-first API](docs/decisions/0002-async-first-api.md), and
  [raw SQL and portability](docs/decisions/0003-raw-sql-and-portability.md)
- [Contribution guide](CONTRIBUTING.md)

Licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
