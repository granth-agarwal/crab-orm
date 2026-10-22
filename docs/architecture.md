# Architecture

Phase 0 implements contracts and repository foundations only. There is no SQL
renderer, connection, database executor, transaction handle, model mapper, or CRUD.

## Responsibilities and dependency direction

```text
sqlite-todo -> crab-orm -> crab-query  -> crab-core
                       -> crab-driver -> crab-core
                       -> crab-core
```

`crab-core` owns `Value`, `CompiledQuery`, and `OrmError` with no dependencies.
`crab-query` owns a limited `Query` input and `QueryCompiler` contract.
`crab-driver` owns `DatabaseExecutor`, `ExecutionResult`, `Row`, and `ColumnValue`.
`crab-orm` deliberately re-exports those nine public types/traits; the example
depends only on that facade. There is no edge between query and driver crates.
Workspace metadata, dependencies, lints, and one generated lockfile are shared as
described in the [Cargo workspace reference](https://doc.rust-lang.org/cargo/reference/workspaces.html).

Future model metadata, primary keys, nullability, and row-to-model conversion
belong to `crab-orm`. Backend row decoding becomes owned values inside the driver;
missing or incompatible model fields become mapping errors in the ORM. No
identity map, change tracking, implicit queries, or lazy loading exists yet.

## Query/compiler boundary

`Query { table: String, columns: Vec<String> }` describes only an unfiltered
single-table projection. It is deliberately temporary preview input, not the
Phase 1 CRUD AST, validated metadata, or a raw SQL wrapper. Its names are
unvalidated. Future implementations must reject empty names/column lists and
validate/quote each name as one identifier according to the selected dialect;
they must never interpret these strings as SQL fragments. Qualified names and
wildcards will require explicit representations later.

`QueryCompiler::compile(&self, &Query) -> Result<CompiledQuery, OrmError>` is
synchronous CPU work, borrows input for one call, and returns owned output. It can
use static dispatch or `dyn QueryCompiler`; thread-sharing bounds are left to
callers. The compiler instance will select/configure a dialect. A future SeaQuery
adapter must translate both input and output without exposing SeaQuery types.
An alternative compiler must meet the same semantics before replacing it.

`CompiledQuery { sql: String, params: Vec<Value> }` makes SQL and ordered bind
values explicitly inspectable. Bind order is placeholder encounter order, never
map iteration order. This struct performs no validation; manually constructing
one does not establish SQL safety or placeholder correctness. A future compiler
must validate identifiers, use placeholders, and return deterministic ordered
values. Executors must bind each value at runtime, never interpolate user data.
Identifiers cannot be bound like values: their validation/quoting is separate.
The current input has no predicates or bind-bearing expressions; those are Phase 1.

## Executor and async contract

`DatabaseExecutor: Send` takes `&mut self` and an owned `CompiledQuery`. `execute`
returns a `Send` future yielding `ExecutionResult { rows_affected: u64 }`; `fetch`
returns a `Send` future yielding all `Vec<Row>` results, empty when none match.
Errors remain `OrmError`. Row columns are an ordered vector of name/value pairs,
so duplicate labels survive decoding; future model mapping must reject ambiguity
or require explicit aliases. Affected-row semantics depend on backend behavior.
Fetch buffers its whole result; streaming, generated keys, and single-row methods
are not provided. Bounded queries will be necessary for large results.

Native return-position `impl Future + Send + '_` keeps generic/static dispatch
allocation-free at the trait boundary. This executor trait is **not dyn
compatible**. It does not require `async-trait`, boxing, `Sync`, or `'static`
executors. A future borrows the executor exclusively until completion/drop and
owns its query; output rows own their data. `Send` allows movement between worker
threads, but borrowing futures cannot simply be spawned as `'static` tasks.
Move an owned executor into a task when that lifecycle is needed.

Tokio is selected for future async adapters. The contracts use only `std` and the
current example is synchronous; a runtime dependency would be unused. Async
avoids a blocking-only service interface but introduces cancellation, lifetime,
scheduling, and runtime integration responsibilities. A sync facade and dynamic
executor dispatch require demonstrated use cases and separate decisions.

## Transactions and cleanup: design only

Beginning a future transaction acquires one connection and returns a handle
owning or exclusively borrowing it. Every statement must use that connection.
Passing statements to a pool cannot provide transaction isolation/atomicity.
Commit and rollback must consume or close the handle, preventing later use.
The connection cannot return to a reusable pool until transaction completion is
confirmed; if state is uncertain, close/discard it.

Callers should explicitly await commit or rollback. Cancellation, errors, and
dropped handles need adapter-specific rollback/cleanup. Rust Drop cannot await
arbitrary async work, so a future adapter must document whether it schedules
rollback or discards the connection, and test cancellation at each lifecycle
stage. Do not promise synchronous rollback completion on drop. Nested
transactions/savepoints are deferred. There is intentionally no transaction
trait or working transaction implementation in Phase 0.

## Values, errors, and inspection

The six owned values cover an initial SQLite-oriented subset, not all database
types; see [conversion and portability policy](portability.md). `Real(f64)` means
Value, CompiledQuery, and Row cannot derive `Eq`. Null is a separate value, not
an absent parameter. `OrmError` distinguishes validation, compile, driver,
mapping, and transaction failure stages with owned diagnostic messages.

Query `Debug`/`Display` hides SQL and all binds, exposing only parameter count.
`Value::Debug` hides payloads, including booleans/numbers and nested row values.
Error `Debug`/`Display` exposes the failure stage and hides diagnostic messages;
`Error::source` exposes no driver chain. Public fields/pattern matching enable
explicit inspection, which can reveal secrets. Future instrumentation must avoid
credentials, bound values, and SQL literals/comments by default. Redaction is a
formatting policy, not memory encryption or a safe logger for arbitrary caller data.

## Implemented versus deferred

| Decision | Phase 0 status | Follow-up |
| --- | --- | --- |
| Owned values, SQL/binds, error stages | Implemented with default redaction tests | Real conversion and binding tests |
| Compiler and executor boundaries | Compilable traits only | Implement/test adapters and richer IR |
| Model/mapping ownership | Documented | Explicit metadata and mapping in Phase 1 |
| Tokio async-first | Accepted, Send futures implemented | Add runtime when execution exists |
| One-connection transaction lifecycle | Documented only | Handle API, cancellation and cleanup tests |
| Capability errors and SQL subset | Policy only; Compile error category exists | Structured capability identifiers/matrix |
| Raw SQL escape hatch | Accepted future policy | Explicit runtime API and security tests |
| Performance/portability claims | No evidence asserted | Reproducible conformance and benchmarks |

See [scope and research](product-scope.md) and ADRs
[0001](decisions/0001-adapter-boundaries.md),
[0002](decisions/0002-async-first-api.md), and
[0003](decisions/0003-raw-sql-and-portability.md).
