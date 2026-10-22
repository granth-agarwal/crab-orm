//! Owned, value-free driver failure categories.

/// Constraint category, independent of a particular driver's error type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConstraintKind {
    /// UNIQUE or primary-key constraint.
    Unique,
    /// NOT NULL constraint.
    NotNull,
    /// Foreign-key constraint.
    ForeignKey,
    /// CHECK constraint.
    Check,
    /// Another database-reported constraint category.
    Other,
}

/// Inspectable driver failure details within [`crate::OrmError::Driver`].
///
/// Contains no SQL, parameter payloads, URLs, or database messages. OrmError's
/// ordinary formatting still exposes only the failure stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DriverError {
    /// Invalid URL or connection configuration.
    Configuration,
    /// Opening the configured database failed.
    Connection,
    /// Explicit connection shutdown failed.
    Close,
    /// Empty, NUL-containing, or lexically malformed input.
    InvalidQuery,
    /// Unsupported statement/placeholder form or execute/fetch result shape.
    UnsupportedQuery,
    /// Positional bind count does not match prepared statement metadata.
    ParameterCount {
        /// Number of expected anonymous parameters.
        expected: usize,
        /// Number of supplied values.
        actual: usize,
    },
    /// A bind cannot be encoded without violating portable value semantics.
    InvalidParameter {
        /// Zero-based bind position, without its value.
        index: usize,
    },
    /// A structured database constraint violation.
    Constraint(ConstraintKind),
    /// fetch_one found no row.
    NotFound,
    /// Preparing or executing SQL failed.
    Execution,
    /// A cell could not be decoded faithfully into the portable value domain.
    RowConversion {
        /// Zero-based result column position.
        column: usize,
    },
}
