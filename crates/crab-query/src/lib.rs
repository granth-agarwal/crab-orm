//! Compiler boundary and a SQLite SeaQuery implementation.
//!
//! All public inputs/outputs are CrabORM-owned. SeaQuery types stay inside the
//! private adapter module; no database execution is performed.

mod sqlite;

pub use sqlite::SqliteCompiler;

use crab_core::{CompiledQuery, OrmError, Query};

/// A synchronous, replaceable compiler selected/configured for one dialect.
///
/// Compilation must preserve bind encounter order and never interpolate values.
/// Invalid input is a validation error; unsupported compilation is a compile
/// error. Identifier validation does not replace dialect-specific quoting or
/// runtime binding. [`SqliteCompiler`] implements the current SQLite subset.
///
/// Supports generic/static dispatch and `dyn QueryCompiler`. No `Send`/`Sync`
/// bound is imposed; callers sharing a compiler across tasks must add them.
/// Input is borrowed only for this call and the result owns its SQL and values.
pub trait QueryCompiler {
    /// Compile an owned domain query into SQL and ordered bind values.
    fn compile(&self, query: &Query) -> Result<CompiledQuery, OrmError>;
}
