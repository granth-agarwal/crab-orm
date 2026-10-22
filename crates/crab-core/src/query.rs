//! Owned statement representations. Construction does not render or execute SQL.

use std::collections::BTreeSet;

use crate::validation::invalid;
use crate::{Column, OrmError, Predicate, Table, ValidationContext, ValidationReason, Value};

/// All columns, or a non-empty ordered list of explicit columns.
///
/// This wrapper keeps an empty explicit projection unrepresentable publicly.
///
/// ```compile_fail
/// use crab_core::Projection;
/// let invalid = Projection { columns: Some(vec![]) };
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Projection {
    columns: Option<Vec<Column>>,
}

impl Projection {
    /// Request all columns, explicitly and without a wildcard identifier.
    pub fn all() -> Self {
        Self { columns: None }
    }

    /// Request a non-empty ordered column list. Repeated projections are allowed.
    pub fn columns(columns: Vec<Column>) -> Result<Self, OrmError> {
        if columns.is_empty() {
            return Err(invalid(
                ValidationContext::Projection,
                ValidationReason::Empty,
            ));
        }
        Ok(Self {
            columns: Some(columns),
        })
    }

    /// Explicit columns in input order; None means all columns, never an empty list.
    pub fn as_columns(&self) -> Option<&[Column]> {
        self.columns.as_deref()
    }
}

/// One paired assignment; a column can never be separated from its value.
///
/// NULL assignments are allowed. Whether the destination column accepts a value
/// requires future schema/mapping/driver validation. Debug redacts value payloads.
#[derive(Clone, Debug, PartialEq)]
pub struct Assignment {
    /// The validated destination column.
    pub column: Column,
    /// Its owned value.
    pub value: Value,
}

fn validate_assignments(
    assignments: &[Assignment],
    context: ValidationContext,
) -> Result<(), OrmError> {
    if assignments.is_empty() {
        return Err(invalid(context, ValidationReason::Empty));
    }
    let mut seen = BTreeSet::new();
    for (index, assignment) in assignments.iter().enumerate() {
        if !seen.insert(assignment.column.as_str().to_ascii_lowercase()) {
            return Err(invalid(
                context,
                ValidationReason::DuplicateColumn { index },
            ));
        }
    }
    Ok(())
}

/// Direction of an ordered SELECT column.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SortDirection {
    /// Ascending order.
    Ascending,
    /// Descending order.
    Descending,
}

/// One ordering key; multiple keys are applied in their vector order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ordering {
    /// Validated ordering column.
    pub column: Column,
    /// Requested direction; no backend-specific NULL ordering is implied.
    pub direction: SortDirection,
}

/// One-table SELECT with optional filter, ordering, limit, and offset.
///
/// Without a filter, all rows are eligible. Without ordering, no stable result
/// order is promised. Pagination is non-negative u64, with zero allowed; backend
/// range limits and offset-only syntax are future compiler responsibilities.
#[derive(Clone, Debug, PartialEq)]
pub struct Select {
    table: Table,
    projection: Projection,
    filter: Option<Predicate>,
    ordering: Vec<Ordering>,
    limit: Option<u64>,
    offset: Option<u64>,
}

impl Select {
    /// Construct an unfiltered SELECT with a validated projection.
    pub fn new(table: Table, projection: Projection) -> Self {
        Self {
            table,
            projection,
            filter: None,
            ordering: Vec::new(),
            limit: None,
            offset: None,
        }
    }

    /// Set the filter, replacing any previous filter. Compose AND/OR explicitly.
    pub fn with_filter(mut self, filter: Predicate) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Set ordered keys, replacing any previous ordering; empty means no ordering.
    pub fn with_ordering(mut self, ordering: Vec<Ordering>) -> Self {
        self.ordering = ordering;
        self
    }

    /// Set the maximum row count, replacing any previous limit. Zero requests none.
    pub fn with_limit(mut self, limit: u64) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set the number of rows to skip, replacing any previous offset.
    pub fn with_offset(mut self, offset: u64) -> Self {
        self.offset = Some(offset);
        self
    }

    /// Source table.
    pub fn table(&self) -> &Table {
        &self.table
    }
    /// Requested projection.
    pub fn projection(&self) -> &Projection {
        &self.projection
    }
    /// Optional predicate, with grouping preserved.
    pub fn filter(&self) -> Option<&Predicate> {
        self.filter.as_ref()
    }
    /// Ordering keys in caller-specified priority order.
    pub fn ordering(&self) -> &[Ordering] {
        &self.ordering
    }
    /// Requested limit, if specified.
    pub fn limit(&self) -> Option<u64> {
        self.limit
    }
    /// Requested offset, if specified, including when no limit is set.
    pub fn offset(&self) -> Option<u64> {
        self.offset
    }
}

/// Single-row INSERT with non-empty, ordered paired assignments.
///
/// Duplicate columns are rejected ignoring ASCII case. Fields and accessors do
/// not permit mutating an already-validated assignment list.
///
/// ```compile_fail
/// use crab_core::{Insert, Table};
/// let invalid = Insert { table: Table::new("todos").unwrap(), assignments: vec![] };
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Insert {
    table: Table,
    assignments: Vec<Assignment>,
}

impl Insert {
    /// Reject empty/default-values inserts and duplicate destination columns.
    pub fn new(table: Table, assignments: Vec<Assignment>) -> Result<Self, OrmError> {
        validate_assignments(&assignments, ValidationContext::Insert)?;
        Ok(Self { table, assignments })
    }
    /// Target table.
    pub fn table(&self) -> &Table {
        &self.table
    }
    /// Paired columns/values in caller order; values require explicit inspection.
    pub fn assignments(&self) -> &[Assignment] {
        &self.assignments
    }
}

/// UPDATE with non-empty assignments and an explicitly chosen row scope.
#[derive(Clone, Debug, PartialEq)]
pub struct Update {
    table: Table,
    assignments: Vec<Assignment>,
    filter: Option<Predicate>,
}

impl Update {
    /// Construct a filtered UPDATE, rejecting empty/duplicate assignments.
    pub fn new(
        table: Table,
        assignments: Vec<Assignment>,
        filter: Predicate,
    ) -> Result<Self, OrmError> {
        validate_assignments(&assignments, ValidationContext::Update)?;
        Ok(Self {
            table,
            assignments,
            filter: Some(filter),
        })
    }
    /// Explicitly request an UPDATE of every row; no implicit filter is added.
    pub fn all_rows(table: Table, assignments: Vec<Assignment>) -> Result<Self, OrmError> {
        validate_assignments(&assignments, ValidationContext::Update)?;
        Ok(Self {
            table,
            assignments,
            filter: None,
        })
    }
    /// Target table.
    pub fn table(&self) -> &Table {
        &self.table
    }
    /// Paired columns/values in caller order; values require explicit inspection.
    pub fn assignments(&self) -> &[Assignment] {
        &self.assignments
    }
    /// None means all_rows was explicitly requested.
    pub fn filter(&self) -> Option<&Predicate> {
        self.filter.as_ref()
    }
}

/// DELETE with an explicitly chosen row scope.
#[derive(Clone, Debug, PartialEq)]
pub struct Delete {
    table: Table,
    filter: Option<Predicate>,
}

impl Delete {
    /// Construct a DELETE restricted by the supplied predicate.
    pub fn new(table: Table, filter: Predicate) -> Self {
        Self {
            table,
            filter: Some(filter),
        }
    }
    /// Explicitly request deletion of every row; no implicit filter is added.
    pub fn all_rows(table: Table) -> Self {
        Self {
            table,
            filter: None,
        }
    }
    /// Target table.
    pub fn table(&self) -> &Table {
        &self.table
    }
    /// None means all_rows was explicitly requested.
    pub fn filter(&self) -> Option<&Predicate> {
        self.filter.as_ref()
    }
}

/// Owned compiler input for Phase 1.1; no SQL or database execution is provided.
#[derive(Clone, Debug, PartialEq)]
pub enum Query {
    /// Read rows.
    Select(Select),
    /// Insert one row.
    Insert(Insert),
    /// Update explicitly scoped rows.
    Update(Update),
    /// Delete explicitly scoped rows.
    Delete(Delete),
}
