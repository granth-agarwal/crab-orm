# ADR 0003: Explicit raw SQL and a tested portability subset

## Status

Accepted policy, 2026-10-22. Owned SQL/binds and redacted formatting are implemented.
Phase 1.3 executes trusted CompiledQuery SQL with a restricted raw subset; raw SQL
convenience APIs, capability APIs, and cross-backend conformance remain deferred.
Phase 1.2 compiles the current owned query subset to SQLite SQL through SeaQuery,
with quoting, ordered binds, and explicit unsupported-value/range errors. Database
execution now has isolated SQLite integration tests; these do not establish
cross-backend database-result parity.

## Context

Applications need SQL features beyond an ORM's initial subset. SQLite, PostgreSQL,
and MySQL differ in types, syntax, and transaction semantics. A universal SQL
claim would hide unsupported behavior. Raw SQL and SQL inspection are already
common ecosystem capabilities, including in the researched tools.

## Decision

Start with SQLite. Establish a documented shared subset/capability matrix through
future cross-backend conformance tests. Unsupported compilation uses
`OrmError::Compile`; invalid input uses Validation errors. Typed capability
identifiers and detailed unsupported-feature variants will be designed when
capabilities exist. Never silently omit unsupported clauses or imply portability
for queries that have not been validated on a backend.

Keep trusted raw SQL at the existing CompiledQuery boundary using SQL plus ordered
owned bind values. Phase 1.3 supports one statement and anonymous `?` placeholders
only, with exact prepared bind counts and execute/fetch result-shape checks.
A lexical guard rejects batching and named/numbered placeholders without parsing
SQL grammar; SQLite validates syntax. See [driver restrictions](../sqlite-driver.md).
The SQLx SqlSafeStr wrapper explicitly marks this trusted boundary, not a sanitizer.
This bypasses compiler portability guarantees; manual CompiledQuery construction
validates nothing. A convenience helper, raw query-IR node, and transaction
integration remain later work. Values must
always be bound at runtime; untrusted SQL fragments must never be composed into
the string. Identifier validation/quoting is dialect work, not parameter binding.

Adopt the [SQLite-oriented initial value policy](../portability.md), including
0/1 boolean binds and distinct NULL. Additional database types need explicit
semantics. Default logging hides parameters and credentials; query formatting
also hides SQL, since raw literals/comments can carry secrets. Explicit inspection
of fields or error messages is the caller's responsibility.

## Consequences

Applications will have an escape route for unsupported queries while accepting
backend lock-in and responsibility for trusted SQL structure. Shared owned values
do not make all types portable. Capability and bind-order tests will be required
for each real adapter, and transaction-bound raw SQL will need additional lifecycle tests. Current SQLite
tests cover binding/injection-shaped data and rejected unsupported raw forms; they
do not establish arbitrary SQL safety, cross-backend portability, or transactions.

## Alternatives considered

- Forbid raw SQL: simpler portability claims, but blocks legitimate backend work.
- Accept unstructured SQL only: loses inspectable binds and encourages unsafe
  composition; ordered runtime parameters remain mandatory.
- Render literal-injected SQL for execution/logging: violates binding and secret
  handling requirements; use SQL plus binds instead.
- Promise identical behavior everywhere: not credible without a defined subset
  and verified capabilities.
- Add all dialects/capabilities now: speculative Phase 2 work; defer until exercised.
