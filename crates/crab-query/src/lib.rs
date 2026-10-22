//! A compiler contract without an implementation or SQL renderer.
//!
//! SeaQuery is a planned private adapter. All query domain types live in
//! crab-core, including the owned Phase 1.1 Query enum accepted by this boundary.

use crab_core::{CompiledQuery, OrmError, Query};

/// A synchronous, replaceable compiler selected/configured for one dialect.
///
/// Compilation must preserve bind encounter order and never interpolate values.
/// Invalid input is a validation error; unsupported compilation is a compile
/// error. No compiler implementation is provided yet. Identifier validation does
/// not replace dialect-specific quoting or runtime binding.
///
/// Supports generic/static dispatch and `dyn QueryCompiler`. No `Send`/`Sync`
/// bound is imposed; callers sharing a compiler across tasks must add them.
/// Input is borrowed only for this call and the result owns its SQL and values.
pub trait QueryCompiler {
    /// Compile an owned domain query into SQL and ordered bind values.
    fn compile(&self, query: &Query) -> Result<CompiledQuery, OrmError>;
}
