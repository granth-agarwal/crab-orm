# SQLite compilation: Phase 1.2

`SqliteCompiler`, exported by `crab-query` and the `crab-orm` facade, implements
`QueryCompiler::compile(&Query) -> Result<CompiledQuery, OrmError>`. The stateless
compiler is Send/Sync. Compilation is synchronous and opens no database connection.
All public inputs, outputs, and errors are CrabORM-owned.

## Usage

```rust
use crab_orm::{OrmError, Projection, Query, QueryCompiler, Select, SqliteCompiler, Table};

fn compile_todos() -> Result<(), OrmError> {
    let query = Query::Select(
        Select::new(Table::new("todos")?, Projection::all()).with_limit(10),
    );
    let compiled = SqliteCompiler.compile(&query)?;
    println!("SQL: {}", compiled.sql);
    println!("Parameter count: {}", compiled.params.len());
    Ok(())
}
```

This produces `SELECT * FROM "todos" LIMIT ?` with one parameter. The
[runnable example](../examples/sqlite-todo/README.md) now compiles and executes
INSERT/SELECT using the optional [SQLite driver](sqlite-driver.md). This compiler
usage snippet alone opens no connection and prints no bind values.

## Mapping, quoting, and bind order

SELECT maps all/explicit projections, source, grouped filters, ordered sort keys,
and limit/offset. INSERT maps paired columns/values for one row. UPDATE maps
assignments followed by its optional filter; DELETE maps its target and optional
filter. Explicit `all_rows` mutations have no WHERE clause. Projection and
assignment order are retained. Qualified identifiers remain unsupported.

Equal, NotEqual, GreaterThan, LessThan, AND, and OR map recursively to SeaQuery
expressions. It may omit redundant parentheses while preserving SQL precedence.
Statement/operator/direction matches are exhaustive. No artificial unsupported
AST variant was added, and no expression is silently replaced or ignored.

Identifiers use SeaQuery's `Alias` APIs and SQLite double quoting, including
keywords; names are never concatenated into SQL or parsed as fragments. Core
validation remains a deliberately limited subset, not a substitute for quoting.

Every statement uses parameterized **build**, never literal-rendering `to_string`.
Bind order comes exclusively from SeaQuery's emitted Values, converted fallibly
into owned values. UPDATE assignment binds precede predicate binds; SELECT
predicate binds precede pagination binds. Each `?` corresponds to one Vec element.
The optional SQLite driver binds those values at runtime in that order.

## Values and failures

Null, Boolean, Integer(i64), finite Real(f64), Text, and Blob retain their variants.
NULL assignments remain bound parameters, distinct from false, zero, and empty
text/blobs. NULL comparisons are rejected by core constructors and defensively
by the compiler. IS NULL/IS NOT NULL are not present in the current IR.
Boolean remains Boolean in CompiledQuery; SQLite execution encodes
it as integer 0/1. This adapter does not perform database encoding.

NaN/infinities and out-of-range pagination return `OrmError::Compile` with static,
value-free context. The return conversion accepts only expected SeaQuery Bool,
BigInt, BigUnsigned pagination, Double, String, and Bytes forms; unexpected types
fail rather than being coerced into strings. Error Display/Debug stays redacted;
CompiledQuery default formatting also hides SQL and values. Explicit inspection
requires care, especially for manually constructed SQL containing literals.

## Offset without limit

Limits/offsets are accepted only in `0..=i64::MAX`; conversion from unsigned values
is checked. Zero and None remain distinct. Pagination values are parameters.
SeaQuery 1.0.2's common builder emits `OFFSET ?` without a LIMIT fallback, while
SQLite requires LIMIT with OFFSET. The adapter emits `LIMIT ? OFFSET ?` with an
internal Integer(-1) bind: SQLite defines a negative limit as **unbounded**.
The caller's None limit is unchanged; no finite user-visible limit is invented.
See [SQLite's LIMIT documentation](https://sqlite.org/lang_select.html#limitoffset).

SeaQuery's limit setter takes only u64. For this case only, the adapter gives it
a private u64::MAX marker, then normalizes that emitted BigUnsigned bind to -1.
Caller pagination is checked first, and no ordinary input value emits BigUnsigned,
so the marker cannot collide with valid input. Exactly one must be emitted for
offset-only compilation or it fails. An explicit caller limit of u64::MAX is
rejected, never normalized. No SQL string patching, assumed placeholder index, or
separate traversal to infer bind order is used. Recheck this when upgrading.

## Dependency verification and remaining limits

Verified **2026-10-22** against official
[SeaQuery documentation](https://docs.rs/sea-query/1.0.2/sea_query/), the
[upstream repository](https://github.com/SeaQL/sea-query), and Cargo-resolved 1.0.2
source: Cargo.toml, value.rs, identifier/expression APIs, statement builders, and
the common/SQLite query builders, especially prepare_select_limit_offset.
SeaQuery is MIT OR Apache-2.0. Cargo generated the lockfile/checksum; no unrelated
dependency was upgraded.

Only crab-query consumes SeaQuery via workspace dependency metadata, with default
features disabled and backend-sqlite enabled. No derives, audit, other backends,
extended types, or runtime integrations are enabled; the resolved feature tree
adds no transitive crates.

SQL/bind goldens and regression tests cover all statements, grouping, quoting,
injection-shaped input, NULL, blobs, numeric/boolean values, pagination and scope.
Reachable compile failures cover non-finite reals and overflow; private conversion
tests cover unexpected bind types and marker mismatches. Tests need no database,
credentials, or network at runtime. They verify output, not schema validity,
constraints, result correctness, transactions, or portability across backends.
SQLx execution is now implemented separately in the optional driver. Model mapping,
raw query-IR nodes, joins, aliases, subqueries, NOT, explicit
NULL predicates, and other later subphases remain deferred.
