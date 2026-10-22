# SQLite TODO: Phase 0 contract inspection

Run from the workspace root:

```sh
cargo run -p sqlite-todo --locked
```

This synchronous binary imports only the `crab-orm` facade. It constructs a
`CompiledQuery` with a fixed SQL string selecting `id`, `title`, and `completed`
from `todos`, with two `?` placeholders and an ordered `Vec<Value>`.
It prints that fixed SQL, the number of binds, and the Phase 1 status message.
It does not print bind values or open a database connection.

This demonstrates ownership and explicit inspection of SQL and parameter count.
There is no SQL compilation, SQL validation, binding/conversion, schema creation,
execution, row mapping, CRUD, or transaction. The type does not verify that its
SQL matches its parameters. SQLite execution and CRUD will arrive in Phase 1.
No runtime is necessary for this contract-only example; future async execution
will use Tokio. A real TODO service remains a future validation target.
