# SQLite TODO: Phase 1.1 query construction

Run from the workspace root:

```sh
cargo run -p sqlite-todo --locked
```

This synchronous binary imports only the `crab-orm` facade and constructs:

- A SELECT of `id`, `title`, `completed`, with `(completed = value OR title = value)
  AND id > value`, ordered by id, with a limit and offset.
- An INSERT with paired title/completed assignments.
- An UPDATE of completed, filtered by id.
- A DELETE filtered by id.

Those expressions describe an owned tree; the program does not render them into
SQL. It wraps the statements in the core `Query` enum and reports the count and
construction status. It does not print bind values. All identifiers pass the
conservative core validation policy. Construction errors propagate as OrmError.

No schema is created, no database is contacted, and no persistence or transaction
occurs. SQL compilation, parameter binding, SQLite execution, model mapping, and
working CRUD remain later work. No runtime or database setup is necessary.
See [query types and invariants](../../docs/query-types.md) for API examples and
the explicitly named full-table mutation constructors.
