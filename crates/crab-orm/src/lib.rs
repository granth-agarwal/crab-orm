//! CrabORM's small public facade. Phase 0 provides contracts, not a working ORM.
//!
//! Future model metadata and row-to-model mapping belong here, including field
//! types, nullability, primary keys, and explicit missing/incompatible-field
//! errors. Models will not need to depend on SQLx or SeaQuery. There are no CRUD
//! helpers, derives, sessions, automatic queries, or transactions in Phase 0.

pub use crab_core::{CompiledQuery, OrmError, Value};
pub use crab_driver::{ColumnValue, DatabaseExecutor, ExecutionResult, Row};
pub use crab_query::{Query, QueryCompiler};
