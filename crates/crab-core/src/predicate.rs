//! Owned column/value comparisons and explicitly grouped logical predicates.

use crate::{Column, OrmError, ValidationContext, ValidationReason, Value, validation::invalid};

/// Supported non-NULL column/value comparison operators.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparisonOperator {
    /// Equal to a value.
    Equal,
    /// Not equal to a value.
    NotEqual,
    /// Greater than a value.
    GreaterThan,
    /// Less than a value.
    LessThan,
}

/// A validated column/value comparison, constructed by [`Predicate::compare`].
///
/// No mutable access is exposed, so its value cannot later be replaced by NULL.
///
/// ```compile_fail
/// use crab_core::{Column, Comparison, ComparisonOperator, Value};
/// let invalid = Comparison {
///     column: Column::new("id").unwrap(),
///     operator: ComparisonOperator::Equal,
///     value: Value::Null,
/// };
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Comparison {
    column: Column,
    operator: ComparisonOperator,
    value: Value,
}

impl Comparison {
    /// The compared column.
    pub fn column(&self) -> &Column {
        &self.column
    }

    /// The comparison operation.
    pub fn operator(&self) -> ComparisonOperator {
        self.operator
    }

    /// Explicitly inspect the owned value; do not log sensitive data.
    pub fn value(&self) -> &Value {
        &self.value
    }
}

/// A recursive predicate preserving grouping and left-to-right child order.
///
/// No evaluation, type checking, SQL generation, NOT, or NULL tests are provided.
/// Debug delegates to Value's payload redaction.
#[derive(Clone, Debug, PartialEq)]
pub enum Predicate {
    /// A validated non-NULL comparison.
    Comparison(Comparison),
    /// Both children must hold; grouping is retained without flattening.
    And(Box<Predicate>, Box<Predicate>),
    /// Either child may hold; grouping is retained without flattening.
    Or(Box<Predicate>, Box<Predicate>),
}

impl Predicate {
    /// Construct a column/value comparison, rejecting NULL for every operator.
    ///
    /// SQL NULL is not an ordinary value for equality/inequality/order tests.
    /// IS NULL and IS NOT NULL are deferred beyond Phase 1.1, not rewritten here.
    /// Non-NULL type compatibility is deferred to model/compiler/driver checks.
    pub fn compare(
        column: Column,
        operator: ComparisonOperator,
        value: Value,
    ) -> Result<Self, OrmError> {
        if matches!(value, Value::Null) {
            return Err(invalid(
                ValidationContext::Comparison,
                ValidationReason::NullComparison,
            ));
        }
        Ok(Self::Comparison(Comparison {
            column,
            operator,
            value,
        }))
    }

    /// Group this predicate with the right-hand predicate using AND.
    pub fn and(self, right: Self) -> Self {
        Self::And(Box::new(self), Box::new(right))
    }

    /// Group this predicate with the right-hand predicate using OR.
    pub fn or(self, right: Self) -> Self {
        Self::Or(Box::new(self), Box::new(right))
    }
}
