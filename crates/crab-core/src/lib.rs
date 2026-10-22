//! Owned domain contracts for CrabORM. No database or SQL builder dependencies.
//! Phase 1.1 adds validated identifiers and query construction, without rendering
//! SQL, evaluating predicates, or executing database operations.
//!
//! SQLite conversions are planned, not implemented: booleans bind as integer 0/1,
//! NULL remains distinct from false, zero, empty text, and empty blobs. SQLite
//! integers need model metadata to distinguish booleans when mapping rows.
//! This initial value set does not cover every database type.

use std::{error::Error, fmt};

mod identifier;
mod predicate;
mod query;
mod validation;

pub use identifier::{Column, Table};
pub use predicate::{Comparison, ComparisonOperator, Predicate};
pub use query::{
    Assignment, Delete, Insert, Ordering, Projection, Query, Select, SortDirection, Update,
};
pub use validation::{ValidationContext, ValidationError, ValidationReason};

/// An owned value, independent of a driver or compiler's type system.
///
/// `Debug` redacts all non-null payloads. Pattern matching explicitly exposes
/// values for inspection; callers must keep them out of ordinary logs.
#[derive(Clone, PartialEq)]
pub enum Value {
    /// SQL NULL; its intended column type comes from model/query context.
    Null,
    /// A logical boolean; the planned SQLite binding is integer 0 or 1.
    Boolean(bool),
    /// A signed 64-bit integer.
    Integer(i64),
    /// A double precision floating point number; not an exact decimal.
    Real(f64),
    /// Owned UTF-8 text.
    Text(String),
    /// Owned binary data.
    Blob(Vec<u8>),
}

impl fmt::Debug for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("Null"),
            Self::Boolean(_) => f.write_str("Boolean([redacted])"),
            Self::Integer(_) => f.write_str("Integer([redacted])"),
            Self::Real(_) => f.write_str("Real([redacted])"),
            Self::Text(_) => f.write_str("Text([redacted])"),
            Self::Blob(_) => f.write_str("Blob([redacted])"),
        }
    }
}

/// SQL and bind values in placeholder encounter order.
///
/// This is a data contract, not a validator: constructing it does not check SQL,
/// placeholder counts, dialect compatibility, or safety. Future compilers and
/// executors must enforce those rules. Both display formats hide SQL and values;
/// SQL itself can contain literals or comments with confidential information.
#[derive(Clone, PartialEq)]
pub struct CompiledQuery {
    /// SQL with dialect-specific placeholders, available for explicit inspection.
    pub sql: String,
    /// Ordered binds; values must never be interpolated into SQL.
    pub params: Vec<Value>,
}

impl fmt::Debug for CompiledQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CompiledQuery")
            .field("sql", &"[redacted]")
            .field("parameter_count", &self.params.len())
            .finish()
    }
}

impl fmt::Display for CompiledQuery {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "CompiledQuery ({} parameters; SQL redacted)",
            self.params.len()
        )
    }
}

/// Distinguishable failure stages, with explicitly accessible diagnostics.
///
/// `Display` and `Debug` expose only the failure stage. Non-validation messages can contain
/// driver details, credentials, or values; inspect them only in trusted contexts.
/// Adapter error types are deliberately absent from this boundary.
#[derive(Clone, PartialEq, Eq)]
pub enum OrmError {
    /// Invalid caller input or identifiers.
    Validation(ValidationError),
    /// SQL compilation failure, including unsupported dialect capabilities.
    Compile(String),
    /// Connection, binding, execution, or decoding failure in a driver.
    Driver(String),
    /// A row could not be converted into the requested model.
    Mapping(String),
    /// Beginning, completing, or cleaning up a transaction failed.
    Transaction(String),
}

impl OrmError {
    fn summary(&self) -> &'static str {
        match self {
            Self::Validation(_) => "validation failure",
            Self::Compile(_) => "compile failure",
            Self::Driver(_) => "driver failure",
            Self::Mapping(_) => "mapping failure",
            Self::Transaction(_) => "transaction failure",
        }
    }
}

impl fmt::Display for OrmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.summary())
    }
}

impl fmt::Debug for OrmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("OrmError")
            .field("stage", &self.summary())
            .field("message", &"[redacted]")
            .finish()
    }
}

impl Error for OrmError {}

#[cfg(test)]
mod tests {
    use super::{
        CompiledQuery, OrmError, ValidationContext, ValidationError, ValidationReason, Value,
    };

    #[test]
    fn query_formatting_redacts_sql_and_binds_but_explicit_inspection_preserves_order() {
        let query = CompiledQuery {
            sql: "SELECT confidential_literal WHERE title = ? AND id = ?".into(),
            params: vec![Value::Text("private-title".into()), Value::Integer(984_321)],
        };

        for output in [
            format!("{query}"),
            format!("{query:?}"),
            format!("{query:#?}"),
        ] {
            for secret in ["confidential_literal", "private-title", "984321"] {
                assert!(!output.contains(secret));
            }
            assert!(output.contains('2'));
        }
        assert!(query.sql.contains("title = ? AND id = ?"));
        assert_eq!(query.params[0], Value::Text("private-title".into()));
        assert_eq!(query.params[1], Value::Integer(984_321));
    }

    #[test]
    fn nested_values_do_not_leak_through_debug() {
        let values = vec![
            Value::Boolean(true),
            Value::Integer(984_321),
            Value::Real(9876.125),
            Value::Text("private-title".into()),
            Value::Blob(vec![123, 234]),
        ];
        let output = format!("{values:?}");
        for secret in ["true", "984321", "9876.125", "private-title", "123", "234"] {
            assert!(!output.contains(secret));
        }
    }

    #[test]
    fn error_formatting_preserves_failure_stage_without_diagnostic_secrets() {
        let secret = "password=private-title";
        let errors = [
            (
                OrmError::Validation(ValidationError {
                    context: ValidationContext::Comparison,
                    reason: ValidationReason::NullComparison,
                }),
                "validation",
            ),
            (OrmError::Compile(secret.into()), "compile"),
            (OrmError::Driver(secret.into()), "driver"),
            (OrmError::Mapping(secret.into()), "mapping"),
            (OrmError::Transaction(secret.into()), "transaction"),
        ];
        for (error, stage) in errors {
            for output in [
                format!("{error}"),
                format!("{error:?}"),
                format!("{error:#?}"),
            ] {
                assert!(output.contains(stage));
                assert!(!output.contains(secret));
            }
        }
    }
}
