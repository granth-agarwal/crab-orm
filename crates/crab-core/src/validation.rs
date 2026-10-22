//! Structured, value-free diagnostics for query construction.

use crate::OrmError;

/// The construction boundary that rejected input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationContext {
    /// A table identifier.
    TableIdentifier,
    /// A column identifier.
    ColumnIdentifier,
    /// A column/value comparison.
    Comparison,
    /// An explicit SELECT projection.
    Projection,
    /// INSERT assignments.
    Insert,
    /// UPDATE assignments.
    Update,
}

/// Why construction failed, without echoing names or bind values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ValidationReason {
    /// A required name or collection was empty.
    Empty,
    /// An identifier did not start with an ASCII letter or underscore.
    InvalidStart,
    /// An identifier contained a disallowed byte after its first byte.
    InvalidCharacter {
        /// Zero-based byte position of the first disallowed byte.
        byte_index: usize,
    },
    /// Ordinary comparisons cannot use SQL NULL.
    NullComparison,
    /// An assignment column repeats an earlier name, ignoring ASCII case.
    DuplicateColumn {
        /// Zero-based position of the repeated assignment.
        index: usize,
    },
}

/// Inspectable validation context and reason within [`OrmError::Validation`].
///
/// Contains no rejected input. OrmError's ordinary formatting still exposes only
/// its failure stage. This payload is data, not a separate error hierarchy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ValidationError {
    /// Which input was being constructed.
    pub context: ValidationContext,
    /// The invariant that was violated.
    pub reason: ValidationReason,
}

pub(crate) fn invalid(context: ValidationContext, reason: ValidationReason) -> OrmError {
    OrmError::Validation(ValidationError { context, reason })
}
