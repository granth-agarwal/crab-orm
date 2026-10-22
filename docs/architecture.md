# Architecture

Phase 0 foundations and Phase 1.1 owned query types are implemented. There is no
SQL renderer, connection, executor implementation, transaction handle, model
mapper, or CRUD persistence.

## Responsibilities and dependency direction

```text
sqlite-todo -> crab-orm -> crab-query  -> crab-core
                       -> crab-driver -> crab-core
                       -> crab-core
```

`crab-core` owns all domain types: identifiers, predicates, query representations,
`Value`, `CompiledQuery`, and `OrmError`, with no dependencies.
`crab-query` owns only the `QueryCompiler` contract accepting core `Query` input.
`crab-driver` owns `DatabaseExecutor`, `ExecutionResult`, `Row`, and `ColumnValue`.
`crab-orm` deliberately re-exports the public domain contracts and traits; the example
depends only on that facade. There is no edge between query and driver crates.
Workspace metadata, dependencies, lints, and one generated lockfile are shared as
described in the [Cargo workspace reference](https://doc.rust-lang.org/cargo/reference/workspaces.html).

Future model metadata, primary keys, nullability, and row-to-model conversion
belong to `crab-orm`. Backend row decoding becomes owned values inside the driver;
missing or incompatible model fields become mapping errors in the ORM. No
identity map, change tracking, implicit queries, or lazy loading exists yet.

## Query/compiler boundary

The Phase 0 public-field projection input has been replaced by core's owned
`Query` enum: Select, Insert, Update, and Delete. Validated `Table` and `Column`
names preserve case and accept only `[A-Za-z_][A-Za-z0-9_]*`. Future compilers must
still quote names according to their dialect; names are never SQL fragments.
Qualification is deferred; `Projection::all()` expresses all columns explicitly.

Private fields and fallible constructors reject empty projections/assignments,
duplicate assignment columns (ignoring ASCII case), and every NULL comparison.
Binary AND/OR nodes retain grouping and child order. SELECT has ordering and
independent optional limit/offset intent; filtered UPDATE/DELETE constructors
require a predicate, and named `all_rows` constructors explicitly omit it.
See [query types](query-types.md) for constructors, validation details, and limitations.

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
The input now owns predicate and assignment values. Stored column/assignment/
ordering vectors preserve caller order; predicate children preserve left-to-right
order. Placeholder allocation and bind collection remain compiler work.

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
mapping, and transaction failure stages. Validation carries structured context/
reason/index data; other failures retain owned diagnostic messages.

CompiledQuery `Debug`/`Display` hides SQL and all binds, exposing only parameter count.
`Value::Debug` hides payloads, including booleans/numbers and nested row values.
Error `Debug`/`Display` exposes the failure stage and hides diagnostic messages;
`Error::source` exposes no driver chain. Public fields/pattern matching enable
explicit inspection, which can reveal secrets. Future instrumentation must avoid
credentials, bound values, and SQL literals/comments by default. Redaction is a
formatting policy, not memory encryption or a safe logger for arbitrary caller data.

## Implemented versus deferred

| Decision | Current status | Follow-up |
| --- | --- | --- |
| Owned values, SQL/binds, error stages | Implemented with default redaction tests | Real conversion and binding tests |
| Owned query IR in core | Implemented through Phase 1.1, with invariant tests | Compiler integration and later query features |
| Compiler and executor boundaries | Compilable traits only | Implement/test adapters |
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
