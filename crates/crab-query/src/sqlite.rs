use crab_core::{
    ComparisonOperator, CompiledQuery, OrmError, Predicate, Query, SortDirection, Value,
};
use sea_query::{
    Alias, Asterisk, Expr, ExprTrait, Order, Query as SeaQuery, SqliteQueryBuilder,
    Value as SeaValue, Values,
};

use crate::QueryCompiler;

/// Compile the owned query IR into SQLite SQL and ordered portable parameters.
///
/// Uses SeaQuery's parameterized build path and quoted identifier APIs. No
/// database is opened. Non-finite reals and pagination outside i64 are rejected.
/// Offset without limit compiles with a bound -1 limit (SQLite's unbounded form),
/// preserving the caller's None limit. NULL assignments remain bound Null values;
/// NULL comparisons remain unsupported by the core constructors.
#[derive(Clone, Copy, Debug, Default)]
pub struct SqliteCompiler;

// SeaQuery 1.0.2 only accepts u64 limits, and does not repair offset-only SQL.
// All caller-provided pagination is checked to fit i64, and no other translated
// input emits BigUnsigned, so this private marker cannot collide with input.
// It is replaced in the emitted bind stream, never by traversing the input IR.
const UNBOUNDED_LIMIT_MARKER: u64 = u64::MAX;

impl QueryCompiler for SqliteCompiler {
    fn compile(&self, query: &Query) -> Result<CompiledQuery, OrmError> {
        // Exhaustive matching: every currently representable statement and
        // predicate is supported. Future IR variants require explicit handling.
        let (built, unbounded_limit) = match query {
            Query::Select(query) => {
                let mut statement = SeaQuery::select();
                statement.from(Alias::new(query.table().as_str()));
                match query.projection().as_columns() {
                    Some(columns) => {
                        statement.columns(columns.iter().map(|column| Alias::new(column.as_str())));
                    }
                    None => {
                        statement.column(Asterisk);
                    }
                }
                if let Some(filter) = query.filter() {
                    statement.and_where(predicate(filter)?);
                }
                for ordering in query.ordering() {
                    let order = match ordering.direction {
                        SortDirection::Ascending => Order::Asc,
                        SortDirection::Descending => Order::Desc,
                    };
                    statement.order_by(Alias::new(ordering.column.as_str()), order);
                }
                let unbounded_limit = query.limit().is_none() && query.offset().is_some();
                if let Some(limit) = query.limit() {
                    check_pagination(limit, "SQLite limit exceeds the portable i64 range")?;
                    statement.limit(limit);
                } else if unbounded_limit {
                    statement.limit(UNBOUNDED_LIMIT_MARKER);
                }
                if let Some(offset) = query.offset() {
                    check_pagination(offset, "SQLite offset exceeds the portable i64 range")?;
                    statement.offset(offset);
                }
                (statement.build(SqliteQueryBuilder), unbounded_limit)
            }
            Query::Insert(query) => {
                let mut statement = SeaQuery::insert();
                statement.into_table(Alias::new(query.table().as_str()));
                statement.columns(
                    query
                        .assignments()
                        .iter()
                        .map(|pair| Alias::new(pair.column.as_str())),
                );
                let values = query
                    .assignments()
                    .iter()
                    .map(|pair| to_sea_value(&pair.value).map(Expr::val))
                    .collect::<Result<Vec<_>, _>>()?;
                // Fallible API, even though paired core assignments already
                // guarantee equal counts. Never forward third-party error data.
                statement.values(values).map_err(|_| {
                    OrmError::Compile("SQLite INSERT column/value count mismatch".into())
                })?;
                (statement.build(SqliteQueryBuilder), false)
            }
            Query::Update(query) => {
                let mut statement = SeaQuery::update();
                statement.table(Alias::new(query.table().as_str()));
                for pair in query.assignments() {
                    statement.value(Alias::new(pair.column.as_str()), to_sea_value(&pair.value)?);
                }
                if let Some(filter) = query.filter() {
                    statement.and_where(predicate(filter)?);
                }
                (statement.build(SqliteQueryBuilder), false)
            }
            Query::Delete(query) => {
                let mut statement = SeaQuery::delete();
                statement.from_table(Alias::new(query.table().as_str()));
                if let Some(filter) = query.filter() {
                    statement.and_where(predicate(filter)?);
                }
                (statement.build(SqliteQueryBuilder), false)
            }
        };
        finish(built, unbounded_limit)
    }
}

fn check_pagination(value: u64, message: &'static str) -> Result<(), OrmError> {
    i64::try_from(value)
        .map(|_| ())
        .map_err(|_| OrmError::Compile(message.into()))
}

fn predicate(input: &Predicate) -> Result<Expr, OrmError> {
    match input {
        Predicate::Comparison(comparison) => {
            if matches!(comparison.value(), Value::Null) {
                return Err(OrmError::Compile(
                    "SQLite NULL comparisons are unsupported".into(),
                ));
            }
            let column = Expr::col(Alias::new(comparison.column().as_str()));
            let value = to_sea_value(comparison.value())?;
            Ok(match comparison.operator() {
                ComparisonOperator::Equal => column.eq(value),
                ComparisonOperator::NotEqual => column.ne(value),
                ComparisonOperator::GreaterThan => column.gt(value),
                ComparisonOperator::LessThan => column.lt(value),
            })
        }
        Predicate::And(left, right) => Ok(predicate(left)?.and(predicate(right)?)),
        Predicate::Or(left, right) => Ok(predicate(left)?.or(predicate(right)?)),
    }
}

fn to_sea_value(value: &Value) -> Result<SeaValue, OrmError> {
    Ok(match value {
        Value::Null => SeaValue::BigInt(None),
        Value::Boolean(value) => SeaValue::Bool(Some(*value)),
        Value::Integer(value) => SeaValue::BigInt(Some(*value)),
        Value::Real(value) => {
            check_real(*value)?;
            SeaValue::Double(Some(*value))
        }
        Value::Text(value) => SeaValue::String(Some(value.clone())),
        Value::Blob(value) => SeaValue::Bytes(Some(value.clone())),
    })
}

fn check_real(value: f64) -> Result<(), OrmError> {
    if value.is_finite() {
        Ok(())
    } else {
        Err(OrmError::Compile(
            "SQLite non-finite Real values are unsupported".into(),
        ))
    }
}

fn finish(
    (sql, values): (String, Values),
    unbounded_limit: bool,
) -> Result<CompiledQuery, OrmError> {
    let mut markers = 0;
    let params = values
        .0
        .into_iter()
        .map(|value| {
            if unbounded_limit
                && matches!(value, SeaValue::BigUnsigned(Some(UNBOUNDED_LIMIT_MARKER)))
            {
                markers += 1;
                Ok(Value::Integer(-1))
            } else {
                from_sea_value(value)
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    if markers != usize::from(unbounded_limit) {
        return Err(OrmError::Compile(
            "SQLite offset-only limit marker mismatch".into(),
        ));
    }
    Ok(CompiledQuery { sql, params })
}

// Accept only variants this adapter emits; unexpected types (including future
// feature-unified SeaQuery extensions) fail instead of coercing into strings.
fn from_sea_value(value: SeaValue) -> Result<Value, OrmError> {
    Ok(match value {
        SeaValue::Bool(value) => value.map_or(Value::Null, Value::Boolean),
        SeaValue::BigInt(value) => value.map_or(Value::Null, Value::Integer),
        SeaValue::BigUnsigned(Some(value)) => {
            Value::Integer(i64::try_from(value).map_err(|_| {
                OrmError::Compile("SQLite unsigned bind exceeds the portable i64 range".into())
            })?)
        }
        SeaValue::BigUnsigned(None) => Value::Null,
        SeaValue::Double(Some(value)) => {
            check_real(value)?;
            Value::Real(value)
        }
        SeaValue::Double(None) => Value::Null,
        SeaValue::String(value) => value.map_or(Value::Null, Value::Text),
        SeaValue::Bytes(value) => value.map_or(Value::Null, Value::Blob),
        _ => {
            return Err(OrmError::Compile(
                "SQLite adapter emitted an unsupported bind type".into(),
            ));
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unexpected_emitted_types_and_unsigned_overflow_fail_without_coercion() {
        for value in [
            SeaValue::Char(Some('x')),
            SeaValue::BigUnsigned(Some(u64::MAX)),
        ] {
            assert!(matches!(from_sea_value(value), Err(OrmError::Compile(_))));
        }
    }

    #[test]
    fn offset_only_marker_must_be_emitted_exactly_once() {
        assert!(matches!(
            finish((String::new(), Values(vec![])), true),
            Err(OrmError::Compile(_))
        ));
        assert!(matches!(
            finish(
                (
                    String::new(),
                    Values(vec![SeaValue::BigUnsigned(Some(UNBOUNDED_LIMIT_MARKER)); 2])
                ),
                true
            ),
            Err(OrmError::Compile(_))
        ));
    }
}
