# Portability policy

Phase 1.3 compiles and executes the current subset on SQLite. Rust and SQLite
are the initial target;
PostgreSQL/MySQL and Python are later work. Portability means a documented subset
and explicit capabilities, not identical SQL syntax or coverage of every type.

## Initial owned values and SQLite conversion

The optional SQLx SQLite adapter implements these bindings and storage-class decoding.
SQLite's storage classes/affinity and lack of a separate boolean storage class
are documented in [SQLite datatypes](https://www.sqlite.org/datatype3.html).

| Owned value | SQLite binding | Decoding/mapping policy |
| --- | --- | --- |
| `Null` | SQL NULL | Preserve as Null; future required model fields must reject it with Mapping errors. NULL is not false, zero, or empty text/blob. |
| `Boolean(bool)` | INTEGER 0/1 | Generic integer decoding stays Integer; future boolean model metadata should map only 0/1, rejecting other integers rather than guessing truthiness. |
| `Integer(i64)` | Signed 64-bit INTEGER | Preserve range; reject incompatible/range-losing conversions. |
| `Real(f64)` | REAL | Approximate binary float, not exact decimal; SQLite compilation and binding reject non-finite values. |
| `Text(String)` | TEXT | Own UTF-8 text; reject decoding that cannot preserve the contract. |
| `Blob(Vec<u8>)` | BLOB | Own bytes; an empty blob remains distinct from NULL. |

SQLite's dynamic storage classes and column affinity can affect stored results.
Integration tests cover actual round trips, dynamic storage classes and expression
columns; decoding checks NULL first and never relies on declared column types.
Raw row integers lack enough information to reconstruct a boolean without
model/type context. Null binds may need contextual type information in
other drivers. Unsupported/invalid driver decoding becomes DriverError::RowConversion;
incompatible model fields become Mapping errors. No silent lossy cross-backend coercion is promised.

The initial set excludes unsigned wide integers, exact decimal, timestamps,
time zones, UUIDs, JSON-specific semantics, arrays, enums, and backend extension
types. Add types only with explicit semantics and adapter round-trip tests.
Floating point contains NaN semantics, so types containing it derive PartialEq
where useful and never Eq.

## SQL subset and capabilities

The compiled SQLite subset is SELECT/INSERT/UPDATE/DELETE, supported column/value
comparisons, AND/OR grouping, SELECT ordering and limit/offset. It is verified by
SQL/bind goldens and isolated SQLite compiler-to-driver tests. This is not evidence
of cross-backend parity or model-level ORM correctness.
Phase 1 targets explicit SQLite insert/read/update/delete, comparisons,
conjunction/disjunction, null checks, ordering, limit/offset, and basic transactions.
Joins/aggregates, upserts, returning clauses, savepoints, backend-specific functions,
and other advanced operations require separate capability decisions/tests.
The presence of hand-written SQL in CompiledQuery does not imply support.

Future compilers must reject unsupported operations with `OrmError::Compile`;
malformed input/identifiers use Validation errors. Structured capability error
details and a tested backend capability matrix are deferred to the portability
phase. Do not silently drop clauses, rewrite with different semantics, or emulate
transactions outside one connection. Placeholder syntax, case/quoting, collation,
NULL comparison/order, generated keys, constraints, isolation, and affected-row
counts must be assessed per backend. A shared suite will establish support.

## Identifiers and values

Dialect compilers must validate and quote table/column identifiers separately
from values, including escaping embedded delimiters according to backend rules.
Names are not SQL fragments and cannot be bound as parameters. Table/Column
constructors validate a conservative ASCII subset; qualification is rejected.
All-column projection has its own explicit representation. Keywords are accepted
and must still be quoted by a dialect compiler. Validation does not replace
quoting or runtime binds. See [query policy](query-types.md) for case-insensitive
assignment duplicate rejection and unsupported NULL comparisons. NULL tests and
schema type checking remain deferred; constructing IR does not validate database semantics.

SQL uses placeholders and an ordered bind vector. The SQLite driver uses runtime
binding in that exact order, never value interpolation. Prepared parameter counts
are checked, and unsupported placeholder/statement forms are rejected. The implemented
compiler uses [SQL plus values from build](https://github.com/SeaQL/sea-query),
not SQL rendered with values injected for debugging. SQLx runtime APIs require
no database at compile time.

## Raw SQL and observability

Manual CompiledQuery SQL executes through the same adapter using trusted SQL plus
owned ordered binds. Its current raw subset is one statement with anonymous `?`
placeholders, optionally followed by one terminator/comments. Named/numbered binds,
batches, NUL, and unterminated quotes/comments are rejected. See
[driver restrictions](sqlite-driver.md). A raw SQL convenience API and future
transaction-bound connection integration are deferred. Raw SQL bypasses compiler
guarantees, cannot safely compose untrusted SQL fragments, and gives up automatic dialect portability. Bound values are still
mandatory for user data. Constructing CompiledQuery itself validates nothing;
the driver checks parameter counts/result shape before execution. No raw SQL convenience method exists.

Default logs must redact parameters and credentials. SQL text can itself contain
secret literals/comments; query formatting hides it, and explicit inspection
requires care. Error messages are inspectable but hidden by their default formats.
Never log connection URLs or blindly forward third-party error chains.
SQL inspection and raw SQL are established ecosystem capabilities, not claimed
CrabORM inventions. See [ADR 0003](decisions/0003-raw-sql-and-portability.md).
