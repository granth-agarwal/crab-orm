# SQLite TODO: Phase 1.3 compiler-to-driver pipeline

Run from the workspace root:

```sh
cargo run -p sqlite-todo --locked
```

The package explicitly enables `crab-orm`'s `sqlite` feature and uses Tokio's
current-thread runtime. It opens one private in-memory SQLite connection, creates
a small todos table using explicit setup SQL, constructs and compiles a portable
INSERT, executes it, then constructs/compiles a filtered SELECT and fetches a row.
The connection is explicitly closed. No database file or service is needed;
a C toolchain builds bundled SQLite and initial dependencies require network access.

All application query/row/error types come through the facade. SQLx and SeaQuery
types stay inside their adapters. The program prints generated SQL, parameter
counts, affected rows, and the fixed non-sensitive fixture's id/title/completed
results. Bind values are not printed. The Boolean bind reads back as Integer(0),
reflecting SQLite's storage classes rather than model metadata.

Model-level CRUD helpers, model mapping, schema management, and transaction APIs
remain future subphases. Explicit CREATE TABLE here is setup, not a migration API.
See [query construction](../../docs/query-types.md),
[SQLite compilation](../../docs/sqlite-compiler.md), and
[driver configuration and limitations](../../docs/sqlite-driver.md).
