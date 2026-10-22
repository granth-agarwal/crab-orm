# CrabORM — Product Requirements Document

**Status:** Proposed · **Version:** 0.1 · **Date:** 2026-10-22\
**Repository:** `crab-orm` · **Product name:** CrabORM\
**Tagline:** A modular, portable SQL ORM powered by Rust.

## 1. Executive summary

CrabORM is an open-source SQL ORM built around a Rust core with replaceable components. It begins as a usable Rust ORM for SQLite, expands to PostgreSQL and MySQL, and later exposes language-specific bindings (Python first). Initially, it composes proven third-party crates behind carefully chosen internal interfaces. It gradually replaces components only when doing so improves portability, maintainability, correctness, ergonomics, or measured performance.

**Positioning:** CrabORM is **not** a promised universal ORM or a blanket faster-than-everything replacement. Its proposed differentiation is (1) portable, inspectable query behavior, (2) a small and predictable core, (3) modular adapters, (4) optional cross-language bindings, and (5) transparent correctness and performance benchmarks. These are product hypotheses to validate, not existing advantages.

### 1.1 Problem

Developers choosing an ORM often trade off portability, convenience, runtime overhead, SQL visibility, and integration with their programming language. Different language ecosystems reinvent similar SQL abstractions, while monolithic ORMs can make it hard to replace individual subsystems. CrabORM explores whether one stable Rust core with thin language adapters can provide a coherent, practical alternative without hiding SQL or sacrificing predictable behavior.

### 1.2 Target users

1. **Rust application developers** who want a lightweight, explicit ORM with conventional CRUD and transactions.
2. **Python backend developers** who want a native-backed ORM interface, once bindings are production-ready.
3. **Library authors** who want to reuse query, dialect, or metadata components separately.
4. **Contributors/learners** who want understandable ORM internals and independently testable modules.

### 1.3 Product promise

- Start simple: schema metadata, typed values, CRUD, parameterized queries, transactions.
- Keep generated SQL and parameters inspectable.
- Support SQLite first; PostgreSQL and MySQL follow after cross-database tests.
- Keep backend-specific features explicit rather than pretending SQL dialects are identical.
- Separate public API, query representation, compiler, driver, ORM session, and language bindings.
- Measure actual performance; do not market unverified speedups.

### 1.4 Explicit non-goals for v0.1

- Replacing SQLx, TLS stacks, or wire-protocol drivers.
- A complete SQLAlchemy/Diesel/SeaORM replacement.
- Automatic migration generation, distributed transactions, sharding, or transparent multi-database joins.
- Arbitrary language support, stable C ABI, or production Python bindings.
- Every database feature behind a universal abstraction.
- Promising faster end-to-end queries without benchmark evidence.

## 2. Why a developer would choose CrabORM

A rational developer will switch only when there is a concrete advantage and acceptable migration risk. The following are **candidate differentiators to earn**:

| Decision factor | What CrabORM must deliver | How to prove it |
|---|---|---|
| Predictable SQL | Stable query representation; compiled SQL and bound parameters can be inspected | Golden SQL tests for each dialect; debug API |
| Small adoption surface | Start with explicit CRUD and opt-in higher-level ORM behavior | Small working example, documented API, low setup burden |
| Cross-database portability | Shared subset of operations across SQLite/PostgreSQL/MySQL; clear capability errors | Same integration test suite against all three DBs |
| Modular design | Replace query compiler without rewriting model/session APIs | Two compiler implementations passing one conformance suite |
| Performance | Lower CPU/allocations on identified workloads, not blanket speed claims | Reproducible comparisons with SQLx, SeaORM, Diesel, SQLAlchemy where appropriate |
| Cross-language consistency | Similar query semantics in Rust and Python | Contract tests with equivalent operations and results |
| Operational trust | Safe parameter binding, transactions, explicit errors, release discipline | Security tests, CI matrix, semver and changelog |

**Competitive reality:** SQLx is a mature SQL toolkit; SeaORM and Diesel already provide sophisticated Rust ORM functionality; SQLAlchemy is mature and highly expressive in Python; Prisma has pioneered shared-engine/client ideas. CrabORM should not compete on feature count at launch. It should first win a narrow niche: **an explicit, inspectable, portable ORM core with replaceable internals**.

### 2.1 Initial wedge and adoption gates

**Wedge:** small Rust services using SQLite that need basic models and CRUD, then the same model/query layer on PostgreSQL. Early adopters must be able to view SQL, use raw SQL when needed, and understand the limitations.

**Do not publicly claim a differentiated production alternative until:**
- CRUD and transaction correctness tests pass across supported DBs.
- Migration/upgrade path and compatibility notes exist.
- Error handling and resource cleanup are documented.
- At least one measured advantage or unusually compelling integration story is demonstrated.
- A real example application is maintained through multiple releases.

## 3. Product architecture

```text
Rust API (v0.1)              Python API (later)           Other languages (future)
       |                            |                               |
       +----------------------------+-------------------------------+
                                    |
                         Public domain API / contracts
                                    |
                     ORM: metadata, mapping, sessions
                                    |
                     Query IR (owned, stable types)
                                    |
                       Query compiler interface
                          /                 \
                  SeaQuery adapter      Native compiler (later)
                          \                 /
                           Compiled SQL + parameters
                                    |
                           Driver interface
                                    |
                            SQLx adapter
                                    |
                     SQLite / PostgreSQL / MySQL
```

**Boundary rules:**
1. Public model and query types must not expose SeaQuery or SQLx types.
2. Use traits only at genuine substitution boundaries; avoid a trait for every helper.
3. Compiler returns SQL **and ordered bind values**; values are never interpolated into SQL.
4. Identifier names must be validated/quoted by dialect rules, not treated as bind parameters.
5. Transactions are bound to one connection; do not silently execute outside their transaction.
6. ORM sessions must not hide unexpected database queries; document eager/lazy behavior explicitly.
7. Rust core should own data and semantics; language bindings should remain thin and explicit.
8. SQL portability is a supported subset plus capabilities, not a guarantee that every feature is portable.

### 3.1 Proposed Cargo workspace

```text
crab-orm/
├── Cargo.toml
├── README.md
├── LICENSE-MIT
├── LICENSE-APACHE
├── crates/
│   ├── crab-core/           # values, errors, metadata, query IR
│   ├── crab-query/          # compiler interface + SeaQuery adapter
│   ├── crab-driver/         # executor interface + SQLx adapter
│   └── crab-orm/            # CRUD, mapping, sessions
├── bindings/
│   └── python/              # added in a later phase
├── examples/
│   ├── sqlite-todo/
│   └── portable-crud/
├── tests/
│   ├── fixtures/
│   ├── conformance/
│   └── integration/
├── benches/
└── docs/
    ├── architecture.md
    ├── portability.md
    └── decisions/
```

**Pragmatism:** begin with these four crates only if separation is helpful; merge crates temporarily rather than maintain artificial boundaries. Add migration, dialect, and bindings crates only when functionality justifies them.

## 4. Third-party dependencies and services

| Dependency | Initial use | Replace later? | Notes |
|---|---|---|---|
| Rust stable + Cargo | Build, workspace, tests | No | MSRV policy defined before first stable release |
| `sea-query` | SQL expression construction and dialect rendering | **Yes, targeted** | Adapter only; avoid leaking its AST into public API |
| `sqlx` | SQLite driver, transactions; later PostgreSQL/MySQL and pool | Generally **no** | Use runtime query APIs; feature-gate database drivers |
| `tokio` | Async runtime | No | Runtime choice is an architectural constraint; document it |
| `thiserror` | Internal/public error definitions | No need | Stable, structured errors |
| `tracing` | Instrumentation | No need | Avoid logging secrets or raw credentials |
| `serde` | Configuration/test fixtures and portable value representations | Optional | Do not make all model semantics JSON-only |
| `uuid` | Optional model field support | Optional | Feature-gated if needed |
| `criterion` | Rust microbenchmarks | No | Add in performance phase |
| `proptest` | Property-based testing | No | Useful for query IR and compiler invariants |
| `pyo3` | Python binding layer (later) | No | Python object conversion cost must be benchmarked |
| `maturin` | Python wheels (later) | No | Build wheels for supported OS/Python versions |

**External services:** None required for the product at runtime. GitHub Actions for CI; GitHub Releases, crates.io, and later PyPI for distribution. Docker or local database containers are used only for integration testing. Do not require a SaaS service or hosted database for core usage.

**Dependency policy:** pin a tested lockfile for workspace applications/examples, review licenses and security advisories, use minimal features, document supported versions, and upgrade deliberately. Verify current crate APIs before implementing; no versions are prescribed by this PRD.

## 5. Core functional requirements

### 5.1 Query core
- Own `Value`, `Column`, `Table`, `Expr`, `Predicate`, `Select`, `Insert`, `Update`, `Delete`, and `CompiledQuery` types.
- Generate parameterized SQL with stable bind order.
- Support comparison, `AND`/`OR`/`NOT`, null checks, sorting, pagination, and basic joins.
- Reject unsupported operations with typed errors; allow explicit raw SQL escape hatch.

### 5.2 Drivers and transactions
- SQLite first, then PostgreSQL and MySQL.
- Connect, execute, fetch, begin, commit, rollback, close.
- Handle cancellation, timeouts, and connection failures without leaving connections stuck.
- Pooling may be exposed through SQLx rather than reimplemented.

### 5.3 ORM
- Explicit model/table metadata and primary-key definitions.
- Row mapping and CRUD; optional generated helpers later.
- Relationships only after base CRUD is stable.
- Session/identity map/change tracking introduced in separate, testable steps.

### 5.4 Language bindings
- Rust-native API first.
- Python bindings after query and ORM semantics stabilize.
- Python layer maps Python values/errors into core types and exposes a documented sync or async model (not both by accident).
- Java/other bindings are exploratory until ABI and lifecycle design is validated.

### 5.5 Non-functional requirements
- Correctness before performance; no string interpolation of user data.
- Parameterized queries and safe identifier quoting.
- Deterministic compilation given the same query and dialect.
- Document thread safety and `Send`/`Sync` assumptions.
- Explicit transaction ownership and cleanup.
- Unit, integration, differential, and property tests.
- Observability without exposing SQL parameter secrets by default.
- Supported-platform CI; benchmark results include methodology and environment.

## 6. Development roadmap — atomic tasks

Each checkbox should be independently implementable and reviewable. A phase is complete only when its acceptance gate passes. Phases are sequential by default; tests and documentation accompany every step.

### Phase 0 — Validate scope and establish foundations

**0.1 Product research**
- [ ] Define the v0.1 user persona and one target application (SQLite TODO service).
- [ ] Write three representative user stories: create model, CRUD records, use transaction.
- [ ] Compare minimal workflows in SQLx, SeaORM, Diesel, and SQLAlchemy.
- [ ] Record what each competitor already does well.
- [ ] State three falsifiable product hypotheses.
- [ ] Define what is explicitly excluded from v0.1.

**0.2 Repository setup**
- [ ] Create `crab-orm` repository with README and license choice.
- [ ] Initialize Cargo workspace.
- [ ] Add `crab-core` crate.
- [ ] Add `crab-query` crate.
- [ ] Add `crab-driver` crate.
- [ ] Add `crab-orm` crate.
- [ ] Add a runnable `examples/sqlite-todo` example.
- [ ] Add `.gitignore`, formatting and lint configuration.
- [ ] Enable CI for `cargo fmt --check`.
- [ ] Enable CI for `cargo clippy`.
- [ ] Enable CI for `cargo test`.
- [ ] Write a contribution guide and architecture overview.

**0.3 Design contracts**
- [ ] Define `OrmError` variants for validation, compile, driver, mapping and transaction failures.
- [ ] Define a portable `Value` enum for the initial SQLite-supported types.
- [ ] Define `CompiledQuery { sql, params }`.
- [ ] Define `QueryCompiler` boundary.
- [ ] Define `DatabaseExecutor` boundary.
- [ ] Record ADR: why SeaQuery and SQLx are adapters.
- [ ] Record ADR: async-first or sync-first Rust API (recommend async-first with Tokio).
- [ ] Record ADR: raw SQL escape hatch and portability policy.

**Gate P0:** workspace builds, checks pass, example starts, contracts are documented.

### Phase 1 — First usable SQLite ORM (v0.1 prototype)

**1.1 Portable query types**
- [x] Define table and column identifiers.
- [x] Implement identifier validation.
- [x] Define equality and inequality predicates.
- [x] Define greater-than and less-than predicates.
- [x] Define conjunction and disjunction.
- [x] Define `SELECT` query representation.
- [x] Define `INSERT` query representation.
- [x] Define `UPDATE` query representation.
- [x] Define `DELETE` query representation.
- [x] Define limit and offset options.
- [x] Define ordering options.
- [x] Unit-test each query representation constructor.

**1.2 SeaQuery adapter**
- [ ] Map `SELECT` IR to SeaQuery statements.
- [ ] Map `INSERT` IR to SeaQuery statements.
- [ ] Map `UPDATE` IR to SeaQuery statements.
- [ ] Map `DELETE` IR to SeaQuery statements.
- [ ] Map each supported predicate to SeaQuery expressions.
- [ ] Compile SQLite SQL with bind placeholders.
- [ ] Produce ordered bound values.
- [ ] Test values never appear interpolated in SQL.
- [ ] Add SQL golden tests for all four statement types.
- [ ] Return typed errors for unsupported expressions.

**1.3 SQLx SQLite adapter**
- [ ] Add feature-gated SQLite SQLx dependency.
- [ ] Implement database URL/config handling.
- [ ] Implement connect and close behavior.
- [ ] Implement `execute` for write statements.
- [ ] Implement `fetch_one`.
- [ ] Implement `fetch_all`.
- [ ] Convert portable values into SQLx binds.
- [ ] Convert SQLx rows into portable row values.
- [ ] Add a test using a temporary SQLite database.
- [ ] Add tests for invalid connection, constraint violation and null values.

**1.4 Model metadata and CRUD**
- [ ] Define a model metadata structure with table name.
- [ ] Define column metadata with types and nullability.
- [ ] Define primary-key metadata.
- [ ] Implement insert-one operation.
- [ ] Implement get-by-primary-key operation.
- [ ] Implement update-by-primary-key operation.
- [ ] Implement delete-by-primary-key operation.
- [ ] Implement list/filter operation.
- [ ] Implement row-to-model mapping.
- [ ] Return typed errors for missing/invalid fields.
- [ ] Test CRUD on a TODO model.

**1.5 Transaction basics**
- [ ] Implement begin transaction.
- [ ] Execute operations on the transaction-bound connection.
- [ ] Implement commit.
- [ ] Implement rollback.
- [ ] Test rollback after a failed insert.
- [ ] Test rollback on explicit cancellation/error path.
- [ ] Document transaction lifetime and drop behavior.

**1.6 First developer experience**
- [ ] Write a 10-minute SQLite quickstart.
- [ ] Document generated SQL inspection.
- [ ] Document raw SQL execution.
- [ ] Add example: create, read, update, delete TODO.
- [ ] Add example: two writes in one transaction.
- [ ] Document unsupported features.
- [ ] Tag a prototype release only after CI passes.

**Gate P1:** complete SQLite TODO example with safe CRUD, transaction tests, inspectable SQL, and no known data-loss defect.

### Phase 2 — Portable SQL across three databases

**2.1 Dialect capabilities**
- [ ] Define dialect identifier enum.
- [ ] Define capability flags for returning clauses, upserts, and placeholder styles.
- [ ] Define a typed unsupported-feature error.
- [ ] Document shared SQL subset.
- [ ] Add tests for dialect-specific identifier quoting.

**2.2 PostgreSQL adapter**
- [ ] Enable SQLx PostgreSQL feature.
- [ ] Add PostgreSQL connection configuration.
- [ ] Compile PostgreSQL placeholders and SQL.
- [ ] Bind supported portable values.
- [ ] Decode PostgreSQL rows.
- [ ] Run CRUD conformance tests.
- [ ] Run transaction conformance tests.
- [ ] Document PostgreSQL-specific limitations.

**2.3 MySQL adapter**
- [ ] Enable SQLx MySQL feature.
- [ ] Add MySQL connection configuration.
- [ ] Compile MySQL placeholders and SQL.
- [ ] Bind supported portable values.
- [ ] Decode MySQL rows.
- [ ] Run CRUD conformance tests.
- [ ] Run transaction conformance tests.
- [ ] Document MySQL-specific limitations.

**2.4 Cross-database verification**
- [ ] Define shared fixture schema for all three databases.
- [ ] Run identical CRUD scenarios across all backends.
- [ ] Test null and boolean differences.
- [ ] Test numeric and timestamp round trips.
- [ ] Test unique/foreign-key constraint errors.
- [ ] Test transaction rollback across all backends.
- [ ] Add GitHub Actions database service containers.
- [ ] Publish portability matrix.

**Gate P2:** same documented CRUD and transaction contract passes on SQLite, PostgreSQL and MySQL.

### Phase 3 — ORM behavior beyond CRUD

**3.1 Query expressiveness**
- [ ] Add inner joins.
- [ ] Add left joins.
- [ ] Add aliases.
- [ ] Add projection/selected columns.
- [ ] Add aggregates (`COUNT`, `SUM`) where portable.
- [ ] Add grouping and having.
- [ ] Test generated SQL per dialect.
- [ ] Test result correctness per backend.

**3.2 Relationships**
- [ ] Define one-to-one relationship metadata.
- [ ] Define one-to-many relationship metadata.
- [ ] Define many-to-many join-table metadata.
- [ ] Implement explicit relationship loading.
- [ ] Implement batch eager loading to reduce N+1 queries.
- [ ] Add query-count tests for relationship loading.
- [ ] Document foreign-key and cascade semantics per database.

**3.3 Sessions and identity map**
- [ ] Define session lifecycle and ownership.
- [ ] Implement identity key as table plus primary key.
- [ ] Cache loaded instances within a session.
- [ ] Ensure repeat loads resolve to the same tracked identity semantics.
- [ ] Implement explicit refresh/expire behavior.
- [ ] Test identity-map invalidation after writes.
- [ ] Document memory and concurrency limitations.

**3.4 Change tracking and unit of work**
- [ ] Define model state: new, clean, dirty, deleted.
- [ ] Track field changes.
- [ ] Implement flush order for inserts and updates.
- [ ] Implement flush order for deletes.
- [ ] Execute flush inside a transaction.
- [ ] Roll back failed flushes.
- [ ] Test failure recovery and stale-object behavior.
- [ ] Document limitations of optimistic concurrency.

**Gate P3:** relationship loading and session semantics are correct, documented, and tested for N+1 behavior.

### Phase 4 — Own the query engine

**4.1 Internal AST independence**
- [ ] Audit public types for SeaQuery leakage.
- [ ] Ensure all query forms use CrabORM-owned IR.
- [ ] Define deterministic traversal order for query nodes.
- [ ] Define compiler conformance fixture format.
- [ ] Add property tests for bind-count consistency.

**4.2 Native SQLite compiler**
- [ ] Implement identifier quoting.
- [ ] Implement parameter numbering/placement.
- [ ] Implement `SELECT` generation.
- [ ] Implement `INSERT` generation.
- [ ] Implement `UPDATE` generation.
- [ ] Implement `DELETE` generation.
- [ ] Implement predicates and null checks.
- [ ] Implement ordering and pagination.
- [ ] Implement joins.
- [ ] Compare outputs against SeaQuery adapter on fixtures.
- [ ] Execute both compilers against SQLite and compare results.

**4.3 Native PostgreSQL/MySQL compilers**
- [ ] Implement PostgreSQL quoting and placeholders.
- [ ] Implement MySQL quoting and placeholders.
- [ ] Implement dialect-specific pagination/returning where supported.
- [ ] Run compiler conformance suite for PostgreSQL.
- [ ] Run compiler conformance suite for MySQL.
- [ ] Run database-result differential tests for all backends.

**4.4 Safe switchover**
- [ ] Add compiler selection to test configuration.
- [ ] Run old/new compilers in CI.
- [ ] Benchmark compilation latency and allocations.
- [ ] Record missing capabilities and regressions.
- [ ] Switch default only after conformance gates pass.
- [ ] Retain fallback until stable release validates replacement.
- [ ] Remove SeaQuery dependency only when coverage is sufficient.

**Gate P4:** native compiler passes parity tests, with no parameter-binding regressions; replacement is optional until stable.

### Phase 5 — Python binding and distribution

**5.1 Binding design**
- [ ] Decide Python sync versus async surface; document tradeoffs.
- [ ] Define Python-to-Rust value conversion rules.
- [ ] Define Rust-to-Python result conversion rules.
- [ ] Define exception mapping.
- [ ] Define connection/session lifetime ownership across the boundary.
- [ ] Document GIL and blocking/async interaction constraints.

**5.2 PyO3 implementation**
- [ ] Create Python binding crate.
- [ ] Expose connection configuration.
- [ ] Expose basic query operations.
- [ ] Expose CRUD API.
- [ ] Expose transactions with safe context-manager behavior.
- [ ] Expose generated SQL inspection.
- [ ] Add Python tests for type conversions.
- [ ] Add Python tests for exceptions and cleanup.
- [ ] Add parity tests against Rust examples.

**5.3 Packaging**
- [ ] Add `pyproject.toml` using Maturin.
- [ ] Build local development wheel.
- [ ] Test wheel installation in a clean virtual environment.
- [ ] Add Linux wheel CI.
- [ ] Add macOS wheel CI.
- [ ] Add Windows wheel CI.
- [ ] Add supported Python version matrix.
- [ ] Publish to TestPyPI.
- [ ] Publish to PyPI only after release checklist passes.

**5.4 Adoption documentation**
- [ ] Write Python quickstart.
- [ ] Publish Rust/Python API parity table.
- [ ] Explain differences from SQLAlchemy and SQLModel.
- [ ] Publish a migration guide for the supported subset.
- [ ] Publish compatibility and stability policy.

**Gate P5:** clean install, consistent CRUD semantics, no runtime leaks in binding tests, documented limitations.

### Phase 6 — Benchmark, optimize, and stabilize

**6.1 Benchmark design**
- [ ] Choose hardware and database setup.
- [ ] Define warmup and iteration counts.
- [ ] Define representative dataset sizes.
- [ ] Separate SQL compilation microbenchmarks from DB round-trip tests.
- [ ] Separate row-mapping benchmarks from full ORM tests.
- [ ] Include memory/allocations where measurable.
- [ ] Capture p50/p95 latency and throughput.
- [ ] Publish raw results and benchmark source.

**6.2 Competitor comparisons**
- [ ] Compare Rust query compilation to SeaQuery.
- [ ] Compare Rust CRUD workloads to SQLx baseline.
- [ ] Compare equivalent ORM workloads to SeaORM.
- [ ] Compare equivalent Python workloads to SQLAlchemy after Python release.
- [ ] Verify functional equivalence and matching query counts.
- [ ] Report cases where CrabORM is slower.

**6.3 Targeted optimizations**
- [ ] Profile hot paths before changes.
- [ ] Reduce avoidable allocations in compiler.
- [ ] Optimize row-to-model conversion.
- [ ] Optimize batch inserts/updates where portable.
- [ ] Measure transaction overhead.
- [ ] Measure Python boundary conversion overhead.
- [ ] Reject optimizations that reduce correctness or clarity without meaningful gains.

**6.4 Release hardening**
- [ ] Document public API stability and semver.
- [ ] Add changelog and migration notes.
- [ ] Audit dependency advisories and licenses.
- [ ] Add fuzz/property tests for query compiler edge cases.
- [ ] Test SQL injection resistance via parameter binding.
- [ ] Test transaction cancellation and resource cleanup.
- [ ] Define supported Rust, Python and database versions.
- [ ] Publish a release candidate.
- [ ] Collect feedback from a real example application.
- [ ] Tag v1.0 only when stability gates are met.

**Gate P6:** reproducible benchmark suite, documented supported API, compatibility matrix, and demonstrated adoption value.

### Phase 7 — Optional future expansion (not committed)

**7.1 Schema migrations**
- [ ] Evaluate existing migration crates before building anything.
- [ ] Define migration file/version format.
- [ ] Implement apply and status commands.
- [ ] Implement rollback only where safe and explicitly supported.
- [ ] Test failed migration recovery.

**7.2 Additional language adapters**
- [ ] Select one target language based on demand.
- [ ] Compare C ABI, JNI, IPC and native binding strategies.
- [ ] Define ownership and memory lifecycle contract.
- [ ] Implement a minimal CRUD proof of concept.
- [ ] Run cross-language conformance tests.

**7.3 Advanced capabilities**
- [ ] Evaluate upserts, bulk operations and returning clauses.
- [ ] Evaluate optimistic locking.
- [ ] Evaluate schema introspection.
- [ ] Evaluate connection health checks and instrumentation.
- [ ] Evaluate native connection pool replacement only if evidence warrants it.

## 7. MVP definition and release strategy

**v0.1 MVP:** SQLite, async Rust API, explicit model metadata, safe CRUD, transactions, query inspection, raw SQL escape hatch, tests, and documentation. No relationships or Python binding required.

**v0.2:** PostgreSQL/MySQL portability with a shared conformance suite.\
**v0.3:** relationships and session fundamentals.\
**v0.4:** optional native compiler and differential tests.\
**v0.5:** Python preview bindings.\
**v1.0:** only after stability, portability, operational, and performance evidence justify it. Version numbers are targets, not deadlines.

### MVP acceptance scenarios

1. Developer defines TODO model metadata and inserts a record.
2. Developer retrieves, updates, and deletes it by primary key.
3. Developer filters and orders a list with bound parameters.
4. Developer executes two operations atomically and verifies rollback on failure.
5. Developer inspects generated SQL without exposing sensitive parameter values in logs.
6. Developer runs all examples using only SQLite and Cargo, without an external SaaS dependency.

## 8. Quality, security, and CI gates

- **Every PR:** format, lint, unit tests, documentation/examples check.
- **Query compiler PR:** SQL golden tests, bind-order tests, invalid-identifier tests.
- **Driver PR:** integration tests for connection errors, constraints, transactions and cleanup.
- **ORM PR:** state/identity consistency tests and query-count assertions where relevant.
- **Binding PR:** Python packaging and cross-language parity tests.
- **Release PR:** multi-platform CI, dependency audit, changelog, migration notes, benchmark review.

**Security invariants:** user data is bound, not interpolated; identifiers are quoted/validated; credentials are redacted; raw SQL is explicitly marked unsafe for untrusted composition; transaction ownership is explicit; database privileges are delegated to DB configuration, not bypassed by the ORM.

## 9. Risks and mitigations

| Risk | Why it matters | Mitigation |
|---|---|---|
| Scope explosion | Full ORM semantics are vast | SQLite CRUD MVP; defer advanced features |
| Premature abstraction | Excess traits complicate learning | Only introduce interfaces at replacement boundaries |
| Driver reimplementation | Protocol correctness/security are hard | Keep SQLx initially and likely long-term |
| False portability | Dialects differ in semantics | Capabilities matrix, explicit errors, conformance tests |
| Performance disappointment | Network and Python conversion dominate | Separate benchmarks; optimize measured hot paths |
| Cross-language object mismatch | Identity, lifecycle and async semantics differ | Stable value contract and binding-specific tests |
| Breaking API changes | Discourages adoption | Semver, changelog, preview releases |
| Competing mature ORMs | Feature parity is unrealistic | Narrow wedge, clear SQL, modular architecture |
| Maintainer burden | Many crates/DBs/platforms | Feature gates, small MVP, CI automation |

## 10. Success metrics

**Engineering:**
- 100% of MVP acceptance scenarios pass in CI.
- All supported database adapters pass shared conformance tests.
- Zero known unsafe interpolation paths in public query builders.
- Example app passes from clean checkout with documented commands.

**Developer experience:**
- New user completes SQLite CRUD quickstart in under 15 minutes in a usability test.
- Generated SQL and bound parameter order are inspectable.
- Unsupported portability cases produce actionable errors.

**Performance:**
- Publish reproducible baselines before claiming improvement.
- Identify at least one realistic workload where CrabORM provides a measurable, repeatable advantage or equivalent performance with substantially better modularity/ergonomics.
- Track regressions over releases; do not choose arbitrary speedup claims as release gates.

**Adoption:**
- At least one nontrivial maintained example app.
- External feedback from developers who have compared it with existing alternatives.
- Documented reasons users choose or reject CrabORM; adjust roadmap accordingly.

## 11. First two weeks — recommended implementation sequence

**Week 1: design + foundation**
- [ ] Create repo, license, README and Cargo workspace.
- [ ] Write a one-page architecture ADR.
- [ ] Define `Value`, `OrmError`, `CompiledQuery`.
- [ ] Implement `SELECT` and `INSERT` query IR.
- [ ] Add SeaQuery adapter for SQLite `SELECT`/`INSERT`.
- [ ] Add golden SQL and bind-order tests.
- [ ] Add CI format/lint/test checks.

**Week 2: working vertical slice**
- [ ] Add SQLx SQLite connection adapter.
- [ ] Execute a parameterized insert.
- [ ] Execute a parameterized select.
- [ ] Define TODO model metadata.
- [ ] Map a row into a TODO model.
- [ ] Add update and delete operations.
- [ ] Add transaction commit/rollback.
- [ ] Write and run the complete TODO example.
- [ ] Publish a development progress note and known limitations.

**Rule:** build vertical slices, not all abstractions first. At the end of each week, the repository should demonstrate working functionality.

## 12. Open decisions (resolve before relevant implementation)

1. **Rust API style:** async-first recommended, but decide whether to offer a sync facade later.
2. **Model definition:** explicit metadata first; derive macros only after semantics stabilize.
3. **Primary keys:** initially support integer/text keys; expand with tests.
4. **Schema creation:** use handwritten SQL fixtures initially; defer migrations.
5. **SQL portability:** document a shared subset plus capability flags.
6. **Python API:** choose sync vs async deliberately before bindings phase.
7. **License:** MIT/Apache-2.0 dual license is common in Rust, subject to your preference and dependency review.
8. **Name availability:** check GitHub, crates.io and PyPI before publication; no availability claim is made here.

---

**Final product principle:** CrabORM should be useful **before** it is fully original. Adopt existing components to deliver a correct ORM; replace them only when a tested, maintainable implementation provides a real benefit. The long-term ambition is a modular, cross-language SQL ORM—not rewriting every dependency for its own sake.
