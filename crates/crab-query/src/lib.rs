//! A compiler contract without an implementation or SQL renderer.
//!
//! SeaQuery is a planned private adapter. This preview input will evolve when
//! Phase 1 introduces validated identifiers, predicates, and CRUD query types.

use crab_core::{CompiledQuery, OrmError};

/// Limited Phase 0 input: an unfiltered projection from one table.
///
/// This is not the Phase 1 query AST or a raw SQL escape hatch. Names are owned
/// and unvalidated. A future compiler must reject empty table/projection names
/// and an empty projection, then validate/quote identifiers for its dialect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Query {
    /// One table name, not a SQL fragment or qualified-name expression.
    pub table: String,
    /// Explicit column names in result order; `*` is not a column name.
    pub columns: Vec<String>,
}

/// A synchronous, replaceable compiler selected/configured for one dialect.
///
/// Compilation must preserve bind encounter order and never interpolate values.
/// Invalid input is a validation error; unsupported compilation is a compile
/// error. No compiler implementation is provided in Phase 0.
///
/// Supports generic/static dispatch and `dyn QueryCompiler`. No `Send`/`Sync`
/// bound is imposed; callers sharing a compiler across tasks must add them.
/// Input is borrowed only for this call and the result owns its SQL and values.
pub trait QueryCompiler {
    /// Compile the limited input into owned SQL and ordered bind values.
    fn compile(&self, query: &Query) -> Result<CompiledQuery, OrmError>;
}
