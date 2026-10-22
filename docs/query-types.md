# Portable query types: Phase 1.1

All query/domain types live in `crab-core` and are intentionally re-exported by
`crab-orm`. `crab-query` contains only the compiler interface accepting `&Query`.
The former public-field Phase 0 projection input has been replaced, not retained
as a competing AST. Constructors validate representation invariants only: no SQL
is rendered, predicates are not evaluated, and no database is contacted.

## Identifiers

`Table::new` and `Column::new` own a non-empty name matching
`[A-Za-z_][A-Za-z0-9_]*`. Spelling/case is preserved; no trimming, normalization,
keyword blacklist, or dotted qualification occurs. Whitespace, non-ASCII letters,
SQL fragments, quotes, separators, wildcard names, and NUL are rejected.
`as_str()` exposes the validated spelling; no mutable name accessor exists.

This deliberately narrow subset excludes many legal SQL identifiers. Keywords
such as `select` are accepted. Identifier equality preserves exact case, but
duplicate assignment checks use ASCII case-insensitive comparison as described
below. Future compilers must still quote identifiers for their dialect, enforce
backend restrictions, and bind values separately. Validation replaces neither
quoting nor runtime parameter binding. Qualified names and aliases are deferred.

## Predicates and NULL

`Predicate::compare(column, operator, value)` creates a validated `Comparison`.
Operators are `Equal`, `NotEqual`, `GreaterThan`, and `LessThan`. Comparison fields
are private and have immutable accessors for compiler/inspection use. AND/OR
compose boxed children with `left.and(right)` and `left.or(right)`; the public
enum also allows explicit `And`/`Or` construction from valid predicates. Children
remain in input order; nested grouping is never flattened or reassociated.

Every comparison rejects `Value::Null`, including equality and inequality. No
ordinary `column = NULL` node can be constructed through the public API.
IS NULL, IS NOT NULL, NOT, column-to-column comparisons, and broader expressions
are outside the exact Phase 1.1 checklist and remain deferred. NULL *assignments*
are permitted; column nullability/type compatibility requires future metadata
and adapter checks. Ordinary non-NULL comparisons are not schema type-checked.

## Statements and invariants

| Type | Construction and inspection |
| --- | --- |
| `Projection` | `all()` explicitly requests all columns; `columns(Vec<Column>)` rejects an empty list. `as_columns()` returns None for all, otherwise a non-empty ordered slice. Repeated selected columns are allowed and retained. |
| `Select` | `new(table, projection)` is unfiltered. `with_filter` sets a grouped predicate. `with_ordering` sets an ordered Vec of `Ordering { column, direction }`; direction is Ascending/Descending. `with_limit`/`with_offset` set independent optional u64 values. |
| `Assignment` | A paired `column: Column` and `value: Value`; count mismatches are unrepresentable. Public fields cannot bypass name validation and cannot mutate stored statement slices. |
| `Insert` | `new(table, assignments)` requires a non-empty ordered vector with unique columns. Default-values and multi-row inserts are deferred. |
| `Update` | `new(table, assignments, filter)` requires a filter. `all_rows(table, assignments)` explicitly omits it. Both validate the assignment list. |
| `Delete` | `new(table, filter)` requires a filter. `all_rows(table)` explicitly omits it. |
| `Query` | Owns one Select, Insert, Update, or Delete, ready as input to a future compiler. |

Insert/update duplicate columns are rejected **ignoring ASCII case**, e.g.
`title` and `TITLE` conflict. This conservative initial policy avoids ambiguous
assignments on SQLite; it does not claim identical identifier resolution on every
backend. The first repeated assignment's zero-based index is reported. Input
order and original spelling are otherwise preserved. There are no mutable
statement/projection/assignment-list accessors to bypass validation after construction.

SELECT filter, ordering, limit, and offset setters replace the corresponding
previous setting. Combine predicates explicitly with AND/OR to retain both.
No filter selects all eligible rows. Without ordering, no stable result order is
promised; include suitable keys when deterministic pagination is needed. Empty
ordering is allowed. Limits/offsets accept zero (`with_limit(0)` requests no rows)
and offset can exist without limit. u64 describes non-negative intent, not every
backend's range; future compilers must reject unrepresentable values or render
equivalent supported offset-only syntax, never truncate or silently omit options.

An unfiltered UPDATE/DELETE means every row is eligible. The named `all_rows`
constructors record that intent without inserting a safety filter or performing
an operation. Filtered mutations retain the supplied predicate exactly; its
presence does not prove a small number of rows will match.

## Construction through the facade

This illustrates owned construction only. It follows the same API exercised by
the [runnable sqlite-todo example](../examples/sqlite-todo/src/main.rs).

```rust
use crab_orm::{
    Assignment, Column, ComparisonOperator as Op, Delete, Insert, OrmError,
    Predicate, Projection, Query, Select, Table, Update, Value,
};

fn queries() -> Result<Vec<Query>, OrmError> {
    let table = Table::new("todos")?;
    let id = Column::new("id")?;
    let title = Column::new("title")?;
    let completed = Column::new("completed")?;
    let filter = Predicate::compare(completed.clone(), Op::Equal, Value::Boolean(false))?
        .or(Predicate::compare(title.clone(), Op::NotEqual, Value::Text("archived".into()))?)
        .and(Predicate::compare(id.clone(), Op::GreaterThan, Value::Integer(0))?);
    let select = Select::new(table.clone(), Projection::all()).with_filter(filter);
    let insert = Insert::new(table.clone(), vec![Assignment {
        column: title, value: Value::Text("Read the docs".into()),
    }])?;
    let by_id = Predicate::compare(id, Op::Equal, Value::Integer(1))?;
    let update = Update::new(table.clone(), vec![Assignment {
        column: completed, value: Value::Boolean(true),
    }], by_id.clone())?;
    let delete = Delete::new(table, by_id);
    Ok(vec![Query::Select(select), Query::Insert(insert),
            Query::Update(update), Query::Delete(delete)])
}
```

For deliberate full-table intent, use `Delete::all_rows(table)` or
`Update::all_rows(table, assignments)?`. They construct representations only.

## Errors, inspection, and deferred work

`OrmError::Validation(ValidationError { context, reason })` now carries structured
query-construction diagnostics, replacing the Phase 0 String payload. Context
identifies table/column identifiers, comparisons, projection, insert, or update;
reason identifies empty input, invalid identifier start/byte position, unsupported
NULL comparison, or duplicate assignment index. Rejected input is not copied into
diagnostics. Matching that variant provides details without parsing messages.
Other OrmError stages and CompiledQuery's SQL/ordered-bind contract are unchanged.

Ordinary OrmError Display/Debug continues to reveal only the failure stage.
Query-tree Debug delegates to Value's payload redaction. Explicit value accessors
and assignment fields reveal data deliberately and must not be logged blindly.
Types containing Value derive PartialEq, never Eq, because Value includes f64.

No SeaQuery/SQLx types or dependencies, SQL renderer, compiler implementation,
database adapter, model mapping, or working CRUD persistence are added.
Joins, aliases, subqueries, raw SQL nodes, NULL tests, NOT, derive macros, and
transactions are deferred. Query construction is not proof of compilable SQL,
correct binding, database semantics, or cross-backend portability.
