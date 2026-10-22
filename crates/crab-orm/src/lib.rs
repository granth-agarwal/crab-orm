//! CrabORM's facade for owned queries, SQLite compilation, and optional execution.
//!
//! Future model metadata and row-to-model mapping belong here, including field
//! types, nullability, primary keys, and explicit missing/incompatible-field
//! errors. Models will not need to depend on SQLx or SeaQuery. There are no CRUD
//! persistence helpers, derives, sessions, automatic queries, or transactions yet.

pub use crab_core::{
    Assignment, Column, Comparison, ComparisonOperator, CompiledQuery, ConstraintKind, Delete,
    DriverError, Insert, Ordering, OrmError, Predicate, Projection, Query, Select, SortDirection,
    Table, Update, ValidationContext, ValidationError, ValidationReason, Value,
};
pub use crab_driver::{ColumnValue, DatabaseExecutor, ExecutionResult, Row};
#[cfg(feature = "sqlite")]
pub use crab_driver::{SqliteConfig, SqliteExecutor};
pub use crab_query::{QueryCompiler, SqliteCompiler};
