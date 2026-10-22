//! CrabORM's public facade: contracts and owned query construction, not persistence.
//!
//! Future model metadata and row-to-model mapping belong here, including field
//! types, nullability, primary keys, and explicit missing/incompatible-field
//! errors. Models will not need to depend on SQLx or SeaQuery. There are no CRUD
//! persistence helpers, derives, sessions, automatic queries, or transactions yet.

pub use crab_core::{
    Assignment, Column, Comparison, ComparisonOperator, CompiledQuery, Delete, Insert, Ordering,
    OrmError, Predicate, Projection, Query, Select, SortDirection, Table, Update,
    ValidationContext, ValidationError, ValidationReason, Value,
};
pub use crab_driver::{ColumnValue, DatabaseExecutor, ExecutionResult, Row};
pub use crab_query::QueryCompiler;
