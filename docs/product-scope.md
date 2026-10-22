# Product scope and research

Research date: **2026-10-22**. The authoritative local specification is `PRD.md`
(proposed v0.1, dated 2026-10-22), accepted by the repository owner in place of the
requested filename `crab-orm-prd.md`. This document covers Phase 0 of its roadmap.
The PRD's two-week vertical slice crosses into Phase 1 and is not Phase 0 scope.

This is a documentation review of official sources, not a benchmark or an
execution report. Competitor examples were not run. API names below describe
documented workflows adapted conceptually to the same Todo, not tested snippets.

## Initial user and application

The initial persona is a Rust developer building a small SQLite TODO service.
One local SQLite file serves a small async service with create/list/get/update/delete
operations. The conceptual model throughout is `Todo { id, title, completed }`:
an integer primary key, required text title, and required boolean completed flag.
It needs safe binds, deterministic parameter order, explicit model mapping,
filtering by completion, ordering/pagination, actionable errors, inspectable SQL,
and two writes that either both persist or both roll back. The developer should
not need a hosted database or adapter-specific application types.

The future service can use handwritten schema SQL initially. HTTP framework,
authentication, UI, deployment, and migration tooling are outside this ORM task.
The existing `sqlite-todo` binary demonstrates only a contract, not this service.

## Three user stories

1. **Define a model:** as this developer, I define Todo's primary key, required
   title, and completed flag explicitly and map returned data into owned Rust
   values, with missing/type-incompatible fields reported rather than guessed.
2. **Perform CRUD:** I create an incomplete Todo, read it by id, list incomplete
   Todos in a stable order, change its title/completed state, and delete it. I
   inspect SQL and bind order while keeping values out of ordinary logs.
3. **Use a transaction:** I create two Todos atomically on one connection. If the
   second write fails, neither persists; I can explicitly commit or roll back.

All three are future functional acceptance targets. None is implemented in Phase 0.

## Documented workflow comparison

Every row uses the conceptual Todo above. Tools already support SQL access and
transactions; those are not unique CrabORM capabilities.

| Tool | Define Todo | CRUD | Transaction | Already does well |
| --- | --- | --- | --- | --- |
| SQLx | Define a Rust struct; `FromRow` can map `id`, `title`, `completed`. Write schema SQL separately. | Write parameterized INSERT/SELECT/UPDATE/DELETE SQL; runtime `query`/`query_as`, `.bind`, execute/fetch methods. | Begin on a connection/pool; execute through the transaction's connection, then consume it with commit/rollback. | Explicit SQL, async I/O, row mapping, optional compile-time query checking. It is a SQL toolkit, not an entity ORM. |
| SeaORM | Define/generate a Todo Entity/Model; use ActiveModel field states to represent changes. | ActiveModel insert/update/delete and Entity find/filter APIs execute asynchronously. | Closure transaction scope or explicit begin/commit/rollback; pass the transaction to operations. | Entity conventions, async CRUD, and established transactional workflows. |
| Diesel | Define schema-backed `todos` columns; map a Rust Todo with Queryable/Selectable and an insert representation. | Typed insert/select/update/delete DSL over the schema. | `Connection::transaction` closure commits on success, rolls back on error. | Schema-aware type checking and explicit typed query composition/model mapping. |
| SQLAlchemy 2.0 | Declare a Python Todo subclass of DeclarativeBase with Mapped fields/mapped_column. | Session.add/select, mutate tracked objects, Session.delete; flush/commit writes. | Session.begin context scopes commit/rollback. | Declarative mapping, expressive ORM queries, unit-of-work sessions and lifecycle management. |

Sources for SQLx: [repository](https://github.com/launchbadge/sqlx),
[runtime query](https://docs.rs/sqlx/latest/sqlx/fn.query.html),
[row mapping](https://docs.rs/sqlx/latest/sqlx/fn.query_as.html),
[transactions](https://docs.rs/sqlx/latest/sqlx/struct.Transaction.html).
The supplied repository URL redirected to `transact-rs/sqlx` during research.
Runtime APIs suit future generated SQL without a build-time database; macro query
checking is optional and can use database metadata/offline preparation.

Sources for SeaORM: the supplied [docs root](https://www.sea-ql.org/SeaORM/docs/)
returned 404 to the research fetch, so the working official
[index](https://www.sea-ql.org/SeaORM/docs/index/) and its
[insert](https://www.sea-ql.org/SeaORM/docs/basic-crud/insert/),
[select](https://www.sea-ql.org/SeaORM/docs/basic-crud/select/),
[update](https://www.sea-ql.org/SeaORM/docs/basic-crud/update/),
[delete](https://www.sea-ql.org/SeaORM/docs/basic-crud/delete/), and
[transaction](https://www.sea-ql.org/SeaORM/docs/advanced-query/transaction/)
pages were reviewed (2.0.x documentation).

Sources for Diesel: [getting started](https://diesel.rs/guides/getting-started)
and [Connection::transaction](https://docs.rs/diesel/latest/diesel/connection/trait.Connection.html#method.transaction).
Sources for SQLAlchemy: the requested
[2.0 ORM quickstart](https://docs.sqlalchemy.org/en/20/orm/quickstart.html) and
[2.0 session basics](https://docs.sqlalchemy.org/en/20/orm/session_basics.html).
The site identifies that documentation series as legacy; this comparison is
specifically of the requested 2.0 workflow, not a claim about the newest series.

## Assessment: tradeoffs and candidate opportunities

These are our assessments of fit for the initial persona, not findings from
comparative usability or performance experiments:

- SQLx makes SQL explicit, but the application still supplies entity CRUD patterns
  and model metadata. CrabORM might reduce repetition while retaining inspection.
- SeaORM provides the requested workflows already. Its entity/ActiveModel
  conventions introduce concepts a small service must learn; a smaller explicit
  surface might appeal to some users, at the cost of much less functionality.
- Diesel's typed DSL provides valuable checking but couples application queries to
  its schema/type vocabulary. Its main Connection API is synchronous; choosing
  async integration requires separate evaluation. CrabORM's owned boundaries might
  suit async services wanting replaceable compilers, with fewer static guarantees.
- SQLAlchemy offers mature Python sessions and rich mapping. That is a different
  language and lifecycle model from the Rust-first target; CrabORM will defer the
  associated complexity, and therefore lacks those capabilities.

SQL visibility, modular components, portability, and transactions already exist
in this ecosystem. The candidate opportunity is a useful *combination and adoption
experience* for this narrow application, not invention or superiority. Costs
include extra owned data copies, immature APIs, adapter maintenance, and a very
small supported subset. There is no measured speed advantage or adoption evidence.

## Falsifiable hypotheses: not yet validated

| Hypothesis | Future validation procedure | Pass / fail criterion |
| --- | --- | --- |
| Basic adoption through the facade is practical for the initial persona. | After Phase 1, give five Rust developers unfamiliar with CrabORM a clean SQLite TODO quickstart. Time model definition plus create/get/update/delete using only `crab-orm` as the application's ORM dependency. Record assistance and errors. | Pass if at least 4/5 finish correctly within 15 minutes without maintainer help or direct lower-crate/adapter dependencies; fail otherwise. |
| SQL and ordered parameters can be inspected without public adapter types. | After compilation/execution exist, build a separate facade-only consumer for Todo insert, filtered list, and update. Assert SQL placeholder/bind correspondence and exact ordered values; inspect exported signatures and default logs for leaks. | Pass if all three cases expose owned SQL/Value data, execute correctly, expose no SQLx/SeaQuery types, and leak no bind values in default formatting; fail on any violation. |
| A compiler can be replaced without changing caller code. | Once two real compilers exist, compile the identical facade consumer against each using only a composition/configuration switch. Run the same Todo CRUD/null/bind-order fixtures against SQLite and compare returned rows/errors. | Pass if caller/model/query source is unchanged, both pass every fixture, and results/error categories agree; fail otherwise. SQL strings may differ while preserving semantics. |

No Phase 0 trait definition or successful test run establishes these hypotheses.
Compiler substitution requires real implementations and database-result parity.

## Explicit v0.1 exclusions

The PRD section 1.4 excludes replacing SQLx/TLS/wire drivers; a complete
SQLAlchemy/Diesel/SeaORM replacement; automatic migration generation; distributed
transactions; sharding; transparent multi-database joins; arbitrary language
support; a stable C ABI; production Python bindings; universal support for all
database features; and speed promises without benchmark evidence.

The roadmap also places PostgreSQL/MySQL, relationships, identity maps, change
tracking/unit of work, native compiler replacement, Python packaging, and
performance benchmarking after the SQLite v0.1 prototype. They are deferred here.
Phase 0 additionally excludes even v0.1 CRUD, working SQL compilation, database
connections/adapters, raw SQL execution, schema management, and transactions.

## Foundation dependency/API decisions

Official APIs were reviewed before dependency selection; **no third-party Cargo
dependencies are selected in Phase 0**. Standard library errors, formatting, and
`Future` cover the implemented contracts. The planned choices remain:

- [SeaQuery](https://github.com/SeaQL/sea-query): construction and dialect rendering;
  query `build` returns SQL plus values. A private adapter will translate owned
  inputs/outputs; interpolated debug SQL is not the execution format.
- [SQLx](https://github.com/launchbadge/sqlx): runtime execute/fetch/bind APIs and
  connection transactions. The researched 0.9.0 query API takes `impl SqlSafeStr`;
  future generated SQL integration must audit its safety wrapper requirements.
  No dependency version or integration is frozen by this review.
- Tokio: chosen future runtime, to be added only with actual async execution.
- [Cargo workspaces](https://doc.rust-lang.org/cargo/reference/workspaces.html):
  shared package metadata, dependencies, lints, resolver, and generated lockfile.

Sources are living documents, accessed on the research date above. Reverify APIs,
features, licenses, and advisories when implementing adapters, rather than assuming
these notes prove future compatibility.
