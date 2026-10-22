# Portability policy

Phase 0 executes **no SQL on any backend**. Rust and SQLite are the initial target;
PostgreSQL/MySQL and Python are later work. Portability means a documented subset
and explicit capabilities, not identical SQL syntax or coverage of every type.

## Initial owned values and planned SQLite conversion

This table defines intended adapter policy, not implemented conversion behavior.
SQLite's storage classes/affinity and lack of a separate boolean storage class
are documented in [SQLite datatypes](https://www.sqlite.org/datatype3.html).

| Owned value | Planned SQLite binding | Decoding/mapping policy |
| --- | --- | --- |
| `Null` | SQL NULL | Preserve as Null; required model fields reject it with Mapping errors. NULL is not false, zero, or empty text/blob. |
| `Boolean(bool)` | INTEGER 0/1 | Generic integer decoding stays Integer; boolean model metadata maps only 0/1, rejecting other integers rather than guessing truthiness. |
| `Integer(i64)` | Signed 64-bit INTEGER | Preserve range; reject incompatible/range-losing conversions. |
| `Real(f64)` | REAL | Approximate binary float, not exact decimal; adapter must explicitly handle/reject non-finite values to avoid silent changes. |
| `Text(String)` | TEXT | Own UTF-8 text; reject decoding that cannot preserve the contract. |
| `Blob(Vec<u8>)` | BLOB | Own bytes; an empty blob remains distinct from NULL. |

SQLite's dynamic storage classes and column affinity can affect stored results.
A future adapter must test actual round trips, not infer correctness from these
Rust types. Raw row integers lack enough information to reconstruct a boolean
without model/type context. Null binds may need contextual type information in
other drivers. Unsupported driver decoding becomes a Driver error; incompatible
model fields become Mapping errors. No silent lossy cross-backend coercion is promised.

The initial set excludes unsigned wide integers, exact decimal, timestamps,
time zones, UUIDs, JSON-specific semantics, arrays, enums, and backend extension
types. Add types only with explicit semantics and adapter round-trip tests.
Floating point contains NaN semantics, so types containing it derive PartialEq
where useful and never Eq.

## SQL subset and capabilities

There is currently **no supported executable SQL subset**. The temporary query
input can describe only selected columns from one table and has no implementation.
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
Names are not SQL fragments and cannot be bound as parameters. The Phase 0 Query
strings are not validated; callers must not mistake them for safe SQL builders.
Future public identifier types must make qualified names and wildcards explicit.

SQL must contain placeholders and an ordered bind vector. Future drivers must use
runtime binding in that exact order, never value interpolation. Parameter count,
type compatibility, and unsupported operations need real adapter tests. For the
planned SeaQuery adapter, use [SQL plus values from build](https://github.com/SeaQL/sea-query),
not SQL rendered with values injected for debugging. SQLx runtime APIs will avoid
requiring a database at compile time.

## Raw SQL and observability

A future explicit raw SQL escape hatch will accept trusted SQL plus owned ordered
bind values and use the same executor, including a transaction-bound connection
when applicable. It bypasses compiler guarantees, cannot safely compose untrusted
SQL fragments, and gives up automatic dialect portability. Bound values are still
mandatory for user data. Raw SQL execution is not implemented by constructing a
CompiledQuery today, and no raw SQL convenience method exists.

Default logs must redact parameters and credentials. SQL text can itself contain
secret literals/comments; query formatting hides it, and explicit inspection
requires care. Error messages are inspectable but hidden by their default formats.
Never log connection URLs or blindly forward third-party error chains.
SQL inspection and raw SQL are established ecosystem capabilities, not claimed
CrabORM inventions. See [ADR 0003](decisions/0003-raw-sql-and-portability.md).
