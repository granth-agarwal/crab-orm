//! Owned execution contracts and an optional single-connection SQLite adapter.
//!
//! Future transactions must own or exclusively borrow one acquired connection.
//! Every operation uses that connection; commit/rollback consumes or closes the
//! handle. A pool executor is not a transaction. Drop/cancellation cleanup must
//! prevent unfinished transactions from returning reusable connections to a pool;
//! explicit awaited rollback is preferred. No transaction API exists yet.

use std::future::Future;

use crab_core::{CompiledQuery, OrmError, Value};

#[cfg(feature = "sqlite")]
mod sqlite;
#[cfg(feature = "sqlite")]
pub use sqlite::{SqliteConfig, SqliteExecutor};

/// The outcome of a statement that does not return rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExecutionResult {
    /// Rows affected as reported by the backend; exact semantics can vary.
    pub rows_affected: u64,
}

/// One named value in a returned row.
#[derive(Clone, Debug, PartialEq)]
pub struct ColumnValue {
    /// Result label; duplicate labels are allowed and retained in order.
    pub name: String,
    /// Owned data, redacted by its default debug implementation.
    pub value: Value,
}

/// An owned row retaining result-column order and duplicate labels.
///
/// Access columns by position or iterate the vector. There is no implicit
/// name-to-value lookup: repeated names must not silently overwrite each other.
///
/// Model conversion belongs to the ORM layer; unsupported decoding is a driver
/// error, while incompatible/missing model fields are mapping errors.
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    /// Named values in result-column order, without a driver-specific row type.
    pub columns: Vec<ColumnValue>,
}

/// Async execution with static dispatch and explicitly `Send` futures.
///
/// This native return-position `impl Future` contract is not dyn compatible and
/// requires no boxing or `async-trait`. Implementors and futures must be `Send`;
/// `Sync` is not required. Each future exclusively borrows the executor for its
/// lifetime, owns the query, and returns owned results. It is not necessarily
/// `'static`: spawning requires moving an owned executor into the task.
///
/// Tokio is the SQLite adapter runtime; these contracts are runtime independent.
/// Implementations must bind values at runtime in parameter order, verify
/// compatibility, and report failures. The trait alone provides no safety checks.
pub trait DatabaseExecutor: Send {
    /// Execute a statement without returned rows; errors must not become success.
    fn execute(
        &mut self,
        query: CompiledQuery,
    ) -> impl Future<Output = Result<ExecutionResult, OrmError>> + Send + '_;

    /// Fetch the first matching row; no row is a typed driver NotFound error.
    ///
    /// Additional matches are ignored; use ordering for a deterministic first row.
    fn fetch_one(
        &mut self,
        query: CompiledQuery,
    ) -> impl Future<Output = Result<Row, OrmError>> + Send + '_;

    /// Fetch all rows in result order; an empty vector means no matching rows.
    ///
    /// This buffers the entire result. Streaming is deferred.
    fn fetch_all(
        &mut self,
        query: CompiledQuery,
    ) -> impl Future<Output = Result<Vec<Row>, OrmError>> + Send + '_;
}
