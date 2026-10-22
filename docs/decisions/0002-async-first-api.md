# ADR 0002: Async-first execution with Tokio as the future runtime

## Status

Accepted, 2026-10-22. Native Send-future executor signatures are implemented;
runtime integration, execution, transactions, and cancellation tests are deferred.

## Context

The initial persona builds a small Rust service. Execution must fit async I/O
without blocking the service interface. SQLx supports async execution, but runtime
choice, lifetime ownership, cancellation, and task movement remain design costs.
Pure query compilation does not need an async runtime.

## Decision

Select Tokio for future adapters. Keep Phase 0 runtime independent and depend only
on `std` until real execution needs Tokio. The example performs no I/O and is
synchronous. Keep compiler methods synchronous.

Use `DatabaseExecutor: Send` methods returning native
`impl Future<Output = Result<..., OrmError>> + Send + '_`, with `&mut self` and an
owned query. This stable-language design explicitly promises Send futures using
[return-position impl Trait](https://doc.rust-lang.org/reference/types/impl-trait.html).
It uses static dispatch and is not dyn compatible. No boxing or async-trait is
needed. Sync and `'static` are not required; futures borrow one executor exclusively
and return owned results. A spawned task needs to own its executor to meet the
task's lifetime requirements. Non-Send executors are outside this initial contract.

Future transactions must own/exclusively borrow one acquired connection. All
operations route through that connection; commit/rollback consumes or closes the
handle. A pool executor is not a transaction. Prefer explicit awaited rollback.
Drop cannot await cleanup, so an adapter must schedule cleanup safely or discard
the connection, never return an uncertain transaction to a reusable pool. Test
error, cancellation, drop, and failed-commit paths before claiming correctness.
No transaction implementation or API is invented in Phase 0.

## Consequences

Callers use awaitable execution and generic executor parameters. Runtime choice
constrains integration; async APIs add cancellation/lifetime complexity and do not
guarantee better throughput for SQLite. Mutable borrowing serializes operations
on one handle. Buffering fetch results is simple but can consume substantial
memory. Dynamic dispatch, streaming, sync wrappers, savepoints, and runtime
alternatives require concrete use cases and separate design work.

## Alternatives considered

- Sync-first execution: straightforward for local scripts, but requires blocking
  integration for async services; not the selected initial service API.
- Offer sync and async simultaneously: doubles lifecycle/API obligations too soon.
- Bare async fn in public traits: concise, but does not express the desired Send
  promise as directly as the chosen explicit future bound.
- Boxed futures/async-trait: useful for dyn dispatch, but adds allocation or a
  dependency without a Phase 0 need.
- Pool-level transaction marker: cannot ensure all statements share one connection
  and is rejected as an ownership model.
