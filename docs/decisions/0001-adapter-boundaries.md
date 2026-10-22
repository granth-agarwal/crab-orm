# ADR 0001: Own the boundaries; keep compilers and drivers replaceable

## Status

Accepted, 2026-10-22. Owned contracts and facade are implemented in Phase 0;
SeaQuery and SQLx adapters and substitution conformance are deferred.
Phase 1.1 replaces the temporary query input with owned domain types in core.

## Context

The Rust/SQLite TODO target needs inspectable SQL and predictable errors. The PRD
proposes proven third-party SQL construction/execution while preserving the
option to replace a compiler without rewriting callers. Toolkits already provide
modularity and SQL inspection; this is a boundary choice, not a uniqueness claim.

## Decision

Core owns values, SQL plus ordered binds, and errors. Query and driver depend only
on core; neither depends on the other. The facade re-exports intended contracts
and will own model metadata/mapping. Public signatures contain no SQLx or SeaQuery
types. Phase 1.1 replaces the temporary query input with core's Query enum and
validated identifiers, predicates, and statement types. The query crate owns the
compiler boundary alone. Construction validates structural invariants, not SQL
semantics. Later query features must follow their roadmap scope.

A future [SeaQuery](https://github.com/SeaQL/sea-query) compiler adapter translates
owned input into dialect-specific SQL plus values. Identifiers must be validated
and quoted for that dialect. A future [SQLx](https://github.com/launchbadge/sqlx)
adapter binds values at runtime and decodes owned rows; no compile-time database
is required. Neither adapter is added as a dependency before it is implemented.
User values must never be interpolated into SQL. Unsupported operations produce
explicit errors. Default diagnostics must redact parameters and credentials.

## Consequences

Callers can depend on the facade alone. Third-party lifetimes/types stay inside
adapters, while owned conversion may cost copies/allocations and requires semantic
mapping tests. Dependency/API upgrades still need review: the researched SQLx
0.9.0 runtime query input has a `SqlSafeStr` bound. No performance benefit follows
from these boundaries. Compiler replacement remains unproven until two real
implementations pass equivalent database-result fixtures without caller changes.
Preview contract changes are expected as types mature.

## Alternatives considered

- Expose SeaQuery's AST and SQLx's row/error types: less translation initially,
  but caller code becomes tied to those libraries and their lifetimes.
- Implement a native SQL renderer/driver immediately: increases correctness and
  security work before validating adoption. Protocol driver replacement is not v0.1 scope.
- One crate with no boundaries: fewer manifests, but weaker independently usable
  contracts. Four small crates match the requested workspace and remain acyclic.
- Add traits for every helper: unnecessary abstraction; traits are confined to
  genuine compiler/executor substitution points.
