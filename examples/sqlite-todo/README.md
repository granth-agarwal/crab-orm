# SQLite TODO: Phase 1.2 SQL compilation

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

The program wraps these statements in the core `Query` enum, compiles each with
the facade's `SqliteCompiler`, and prints generated SQLite SQL plus parameter
counts. Identifiers are validated and quoted; bind values are not printed.
Parameters include predicate/assignment values and SELECT pagination, in emitted
placeholder order. Construction/compilation errors propagate as OrmError.

No schema is created, no database is contacted, and no persistence or transaction
occurs. SQL is compiled but never prepared/bound/executed on a connection.
SQLite execution, model mapping, and working CRUD remain later work. No runtime
or database setup is necessary; initial Cargo dependency downloads need network.
See [query types and invariants](../../docs/query-types.md) for API examples and
the explicitly named full-table mutation constructors, and
[compiler behavior](../../docs/sqlite-compiler.md) for NULL/pagination policies.
