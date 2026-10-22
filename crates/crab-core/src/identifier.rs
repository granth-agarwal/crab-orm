//! Conservative identifiers; dialect-specific quoting remains compiler work.

use crate::{OrmError, ValidationContext, ValidationReason, validation::invalid};

fn validate(name: &str, context: ValidationContext) -> Result<(), OrmError> {
    let mut bytes = name.bytes();
    let Some(first) = bytes.next() else {
        return Err(invalid(context, ValidationReason::Empty));
    };
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return Err(invalid(context, ValidationReason::InvalidStart));
    }
    for (index, byte) in bytes.enumerate() {
        if !(byte.is_ascii_alphanumeric() || byte == b'_') {
            return Err(invalid(
                context,
                ValidationReason::InvalidCharacter {
                    byte_index: index + 1,
                },
            ));
        }
    }
    Ok(())
}

/// An owned, case-preserving, unqualified table identifier.
///
/// Accepts only `[A-Za-z_][A-Za-z0-9_]*`, not every legal SQL identifier.
/// Keywords are accepted; compilers must still quote for their dialect. A dotted
/// string is rejected, never interpreted as qualification. Fields are private.
///
/// ```compile_fail
/// use crab_core::Table;
/// let invalid = Table("todos; DROP TABLE todos".into());
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Table(String);

impl Table {
    /// Validate and own one table name, without trimming or changing its case.
    pub fn new(name: impl Into<String>) -> Result<Self, OrmError> {
        let name = name.into();
        validate(&name, ValidationContext::TableIdentifier)?;
        Ok(Self(name))
    }

    /// Inspect the original validated spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// An owned, case-preserving, unqualified column identifier.
///
/// Uses the same conservative policy as [`Table`]. A wildcard is represented by
/// [`crate::Projection::all`], never by a column named `*`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Column(String);

impl Column {
    /// Validate and own one column name, without trimming or changing its case.
    pub fn new(name: impl Into<String>) -> Result<Self, OrmError> {
        let name = name.into();
        validate(&name, ValidationContext::ColumnIdentifier)?;
        Ok(Self(name))
    }

    /// Inspect the original validated spelling.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
