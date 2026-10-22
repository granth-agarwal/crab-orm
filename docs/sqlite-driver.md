# SQLite execution: Phase 1.3

`SqliteExecutor` is a single owned SQLx SQLite connection behind `crab-driver`'s
optional `sqlite` feature. `crab-orm` forwards that feature and re-exports
`SqliteConfig` and `SqliteExecutor`. Both crates have empty default features.
There are no public SQLx types, connection pool, model-level CRUD helpers,
transaction APIs, or compile-time SQL queries. Builds need no DATABASE_URL or database.

## Configuration and lifecycle

```rust
use crab_orm::{OrmError, SqliteConfig, SqliteExecutor};

async fn connections() -> Result<(), OrmError> {
    let memory = SqliteExecutor::connect(SqliteConfig::in_memory()).await?;
    memory.close().await?;

    // Literal path, not a URL. Creation is explicitly requested; directories
    // must already exist. Without create_if_missing(true), the file must exist.
    let config = SqliteConfig::file("example.sqlite")?.create_if_missing(true);
    let file = SqliteExecutor::connect(config).await?;
    file.close().await
}
```

`in_memory()` configures private connection-local memory, lost when closed.
`file(path)` takes a literal path using connection options, without URL concatenation.
An empty path, NUL, or the special `:memory:` path is rejected; use `in_memory()`
for memory. Invalid/inaccessible paths fail on connect without creating directories.
`from_url("sqlite::memory:")` and SQLite file URLs are also supported through SQLx's
fallible options parser. A `sqlite:` scheme and non-empty filename are required;
SQLx validates its supported URL options. URL `mode=rwc` explicitly requests file
creation; otherwise creation defaults to false. `create_if_missing(bool)` overrides
that policy. URLs are trusted connection configuration, not logged diagnostics.

Foreign-key enforcement is explicitly enabled on every connection. Arbitrary
trusted setup SQL could change it; callers must preserve the policy. SQLx statement
logging is disabled, and config/executor Debug never reveals paths, URLs, or credentials.

Operations exclusively borrow the connection and return Send futures. Use Tokio;
the example uses its current-thread runtime. SQLx performs SQLite work on its worker
thread. `close(self).await` consumes the executor and awaits worker shutdown, so a
closed handle cannot be reused. Drop initiates SQLx cleanup but cannot promise
synchronous completion. Prefer explicit close before deleting files. Cancellation
can leave a statement's outcome unknown; no cancellation atomicity or transaction
rollback guarantee is offered. Transaction ownership/cleanup remains a later subphase.

## Execution and fetching

All methods accept owned `CompiledQuery { sql, params }`:

- `execute` accepts statements with no result columns, including INSERT, UPDATE,
  DELETE and setup CREATE TABLE. It returns `ExecutionResult { rows_affected }`.
- `fetch_one` requires a row-returning statement. It returns the first row; additional
  matches are ignored, so order/limit explicitly for deterministic selection. No row
  returns `OrmError::Driver(DriverError::NotFound)`.
- `fetch_all` buffers all rows in result order. No matches is a successful empty vector.

Execute rejects result-producing statements, and fetch rejects statements with no
result columns, using prepared metadata rather than a SQL statement classifier.
The compiler-to-driver path covers all four portable statement types, including
filters, ordering/pagination, and explicitly unfiltered mutations. See the
[runnable example](../examples/sqlite-todo/README.md).

Affected rows follow SQLite's direct statement count, excluding trigger/cascade
side effects; no generated IDs are fabricated. SQLite can retain the prior DML
count for DDL. The adapter queries `total_changes()` before and after execution to
normalize non-changing statements to zero while otherwise retaining SQLx's direct
count. These two constant internal queries add overhead; no performance claim is made.

## SQL boundary and placeholders

The compiler supplies quoted identifiers and anonymous `?` placeholders. The driver
uses SQLx runtime preparation/binding, never interpolation or compile-time macros.
SQLx 0.9's `AssertSqlSafe` wrapper marks the existing CompiledQuery trusted-SQL
boundary; it is not a sanitizer. Manually supplied SQL must have trusted structure.
Values belong only in the separate parameter vector.

The accepted raw subset is one statement with anonymous `?` binds. One trailing
semicolon and whitespace/comments are allowed. A lexical guard distinguishes
quotes, escaped quote delimiters, bracket identifiers, and comments; it rejects
batches, repeated terminators, NUL, empty/comment-only input, and unterminated
quotes/block comments. Named (`:name`, `@name`, `$name`) and numbered (`?1`) binds
are unsupported. Unquoted `:`, `@`, and `$` are conservatively rejected; quoted
identifiers/literals may contain them. Trigger bodies with internal semicolons
are outside this subset. This guard does not parse or classify SQL grammar;
SQLite remains responsible for syntax validity. A trailing line comment is valid.

Prepared statement parameter metadata must match both the guard's anonymous count
and `params.len()`. Missing and extra binds fail before executing the statement.
This check is necessary because SQLx's SQLite binder can otherwise leave missing
arguments as NULL. Values are bound exactly in vector order, including compiler
pagination binds. Method result shape is also checked before binding/execution.
There is no raw-SQL convenience method or raw query-IR node; explicit setup SQL
uses the existing CompiledQuery boundary and bypasses compiler portability guarantees.

## Values and rows

| Bound portable value | SQLite runtime result decoding |
| --- | --- |
| Null | Null, checked before any type conversion |
| Boolean | Bound as integer 0/1; reads as Integer, never guessed as Boolean |
| Integer(i64) | INTEGER to Integer, preserving signed 64-bit range |
| finite Real(f64) | REAL to Real, without decimal guarantees |
| Text(String) | TEXT to owned UTF-8 Text; invalid UTF-8 is an error |
| Blob(Vec<u8>) | BLOB to owned Blob, including empty blobs |

NaN/infinities are rejected as InvalidParameter rather than allowing SQLite to
normalize NaN to NULL. SQLite affinity may change storage class on insertion.
Each returned cell is decoded from its **actual runtime storage class**, not the
declared column type, including expression columns and dynamically typed columns.
NULL is distinct from false, zero, empty text, and empty blobs. No database type
coverage beyond these storage classes is claimed. Future model metadata may convert
Boolean fields from integers with explicit rules; there is no such conversion now.

`Row.columns` is an ordered vector of `ColumnValue { name, value }`. Labels, even
duplicate labels, retain their positions. Use indexed access or iterate columns;
there is no implicit name lookup or map that could overwrite duplicates. Future
model mapping must decide how to handle ambiguous labels. Default Value/Row debug
redacts payloads; explicitly matching values is deliberate inspection.

## Errors

`OrmError::Driver` now contains an owned `DriverError`, replacing its earlier
String payload. Match this variant for structured details; SQLx errors never escape.

| DriverError | Meaning |
| --- | --- |
| Configuration / Connection / Close | Invalid options, connect failure, explicit shutdown failure |
| InvalidQuery / UnsupportedQuery | Invalid lexical input or unsupported SQL/placeholder/result shape |
| ParameterCount { expected, actual } | Supplied bind count differs from prepared metadata |
| InvalidParameter { index } | A zero-based bind cannot be encoded faithfully |
| Constraint(ConstraintKind) | Unique, NotNull, ForeignKey, Check, or Other constraint |
| NotFound | fetch_one returned no row |
| Execution | Other prepare/execution failures |
| RowConversion { column } | A zero-based result column cannot be decoded faithfully |

Constraint mapping uses SQLx categories and SQLite numeric result codes, not message
matching. Database messages, input SQL/values, and connection details are discarded
rather than forwarded into errors. OrmError Display/Debug still reports only the
failure stage; explicit DriverError inspection gives categories/positions without
payloads. Future ORM row-to-model incompatibilities belong to OrmError::Mapping;
storage-class decoding failures belong to the driver.

## Verification and limits

Verified on **2026-10-22** using official [SQLx documentation](https://docs.rs/sqlx/latest/sqlx/),
[SQLite connection options](https://docs.rs/sqlx/latest/sqlx/sqlite/struct.SqliteConnectOptions.html),
[SQLite parameter rules](https://www.sqlite.org/lang_expr.html#parameters), and actual
Cargo-resolved SQLx 0.9.0 sources: feature definitions, options parsing, argument
binding, prepare/statement metadata, runtime value/type decoding, error categories,
and connection close/drop. SQLx is MIT OR Apache-2.0. Its minimal enabled features
are sqlite-bundled and runtime-tokio; no macros, TLS, other database backends, or
extension loading features are requested. Cargo.lock can include upstream optional
macro packages even though the active runtime feature graph excludes them. Bundled
SQLite needs a C compiler.
Tokio 1.54.1 supplies the example/test runtime; tempfile 3.27.0 isolates file tests.

Feature-gated driver tests cover temporary files, explicit creation, failures,
constraints/FKs, NULL/types, injection-shaped data, duplicate columns, parameter
counts, and close. Facade integration tests compile and execute all four statement
types. Tests require no external service, credentials, or runtime network access.
CI separately builds/tests libraries with SQLite disabled and verifies their normal
dependency graph excludes SQLx; the example is excluded because it enables sqlite.
These tests do not establish model-level CRUD, transaction correctness, full SQLite
syntax coverage, cross-backend portability, or production readiness.
