# CrabORM

A modular SQL ORM project with a Rust core, starting with Rust and SQLite.
**Current status: Phase 1.3 SQLite compilation and execution; model-level ORM APIs
are still pending.** No production readiness or performance advantage is claimed.

## Implemented

- Owned `Value`, distinguishable `OrmError` failures, and `CompiledQuery` containing
  SQL plus ordered parameters. Default query/error formatting redacts contents.
- Validated table/column identifiers, grouped comparison/AND/OR predicates, and
  owned SELECT/INSERT/UPDATE/DELETE representations in `crab-core`.
- SELECT projections, ordering, and limit/offset options; paired, validated
  mutation assignments and explicit full-table mutation intent.
- A `QueryCompiler` interface and `SqliteCompiler` using a private SeaQuery adapter
  to produce SQLite SQL with placeholders and ordered owned bind values.
- Async `DatabaseExecutor` execute/fetch_one/fetch_all contracts and ordered owned rows.
- Optional SQLx SQLite adapter with one owned connection, explicit configuration/close,
  runtime binds, storage-class decoding, and structured redacted driver failures.
- A `crab-orm` facade and a runnable in-memory compiler-to-driver example.

Remaining in Phase 1: model metadata/mapping, model-level CRUD helpers, transactions,
and a raw SQL convenience API. Tokio runs the SQLite adapter. Only `crab-query`
depends on SeaQuery (1.0.2, SQLite backend only); only `crab-driver` optionally
depends on SQLx (0.9.0, bundled SQLite and Tokio, without macros or other backends).
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
A C toolchain is needed to build bundled SQLite. No SQLite installation, database
service, credentials, DATABASE_URL, or SaaS account is needed.

From the repository root:

```sh
rustup show active-toolchain
cargo check --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
cargo test --workspace --all-features --locked
cargo build --workspace --all-features --locked
cargo build --workspace --exclude sqlite-todo --no-default-features --locked
cargo test --workspace --exclude sqlite-todo --no-default-features --locked
cargo run -p sqlite-todo --locked
git diff --check
```

The example opens private in-memory SQLite, creates a fixture table with explicit
setup SQL, compiles and executes an INSERT, compiles and fetches a SELECT, and
closes the connection. It prints SQL, parameter counts, and fixed non-sensitive
fixture results, never bind payloads. See the
[example guide](examples/sqlite-todo/README.md),
[query construction reference](docs/query-types.md),
[SQLite compiler usage](docs/sqlite-compiler.md), and
[SQLite driver configuration, semantics, and limitations](docs/sqlite-driver.md).

Library default features are empty. Enable execution with
`crab-orm = { path = "crates/crab-orm", features = ["sqlite"] }`
(adjust the path for your application), or `crab-driver`'s `sqlite` feature when
using the driver directly. The example enables it explicitly; excluding the example
from no-default-feature checks prevents Cargo feature unification from masking
the optional boundary. CI also checks the normal library dependency graph for SQLx.

## Workspace

| Path | Responsibility |
| --- | --- |
| `crates/crab-core` | Owned query/domain types, values, compiled queries, errors |
| `crates/crab-query` | Compiler contract and private SQLite SeaQuery adapter |
| `crates/crab-driver` | Async contracts, owned rows, optional SQLx SQLite adapter |
| `crates/crab-orm` | Public facade; future model metadata/mapping |
| `examples/sqlite-todo` | In-memory compilation and execution pipeline |
| `docs` | Scope research, architecture, portability, and decisions |

All packages are unpublished (`publish = false`). Cargo generates the committed
workspace `Cargo.lock`. Tests cover construction invariants, grouping, ordering,
redaction, private construction boundaries, and SQLite SQL/bind goldens. They
also exercise real isolated SQLite connections, constraints, binding, and decoding,
plus all four compiled statements through the facade. They do not establish
model-level ORM or transaction correctness. CI runs both optional-feature modes
on push and pull requests without an external database service.

## Design and contributing

- [Product scope, comparisons, and future validation hypotheses](docs/product-scope.md)
- [Architecture and implementation status](docs/architecture.md)
- [Query types, constructors, and validation rules](docs/query-types.md)
- [SQLite compiler, parameter order, and limitations](docs/sqlite-compiler.md)
- [SQLite driver, configuration, and runtime value conversion](docs/sqlite-driver.md)
- [Portability and SQLite value policy](docs/portability.md)
- ADRs: [adapter boundaries](docs/decisions/0001-adapter-boundaries.md),
  [async-first API](docs/decisions/0002-async-first-api.md), and
  [raw SQL and portability](docs/decisions/0003-raw-sql-and-portability.md)
- [Contribution guide](CONTRIBUTING.md)

Licensed under either [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
