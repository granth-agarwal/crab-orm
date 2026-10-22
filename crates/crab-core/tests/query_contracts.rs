//! Public construction invariants, grouping, ordering, mutation scope, and redaction.

use std::fmt::Debug;

use crab_core::{
    Assignment, Column, ComparisonOperator as Op, Delete, Insert, Ordering, OrmError, Predicate,
    Projection, Query, Select, SortDirection, Table, Update, ValidationContext as Context,
    ValidationError, ValidationReason as Reason, Value,
};

fn table() -> Table {
    Table::new("todos").unwrap()
}

fn column(name: &str) -> Column {
    Column::new(name).unwrap()
}

fn by_id() -> Predicate {
    Predicate::compare(column("id"), Op::Equal, Value::Integer(42)).unwrap()
}

fn assignments() -> Vec<Assignment> {
    vec![
        Assignment {
            column: column("title"),
            value: Value::Text("private-title".into()),
        },
        Assignment {
            column: column("completed"),
            value: Value::Boolean(false),
        },
    ]
}

fn invalid<T: Debug>(result: Result<T, OrmError>, context: Context, reason: Reason) {
    assert_eq!(
        result.unwrap_err(),
        OrmError::Validation(ValidationError { context, reason })
    );
}

#[test]
fn identifiers_preserve_case_and_accept_keywords_without_a_blacklist() {
    for name in ["todos", "Todo_2", "_", "_ID", "select", "ORDER"] {
        assert_eq!(Table::new(name).unwrap().as_str(), name);
        assert_eq!(Column::new(name).unwrap().as_str(), name);
    }
}

#[test]
fn identifiers_reject_empty_invalid_starts_and_sql_syntax_with_context() {
    for (name, reason) in [
        ("", Reason::Empty),
        ("9todos", Reason::InvalidStart),
        (" todos", Reason::InvalidStart),
        ("*", Reason::InvalidStart),
        ("\0todos", Reason::InvalidStart),
        ("é", Reason::InvalidStart),
        ("\"todos\"", Reason::InvalidStart),
        ("'todos'", Reason::InvalidStart),
        ("`todos`", Reason::InvalidStart),
        ("[todos]", Reason::InvalidStart),
        ("todos ", Reason::InvalidCharacter { byte_index: 5 }),
        ("todos\t", Reason::InvalidCharacter { byte_index: 5 }),
        ("todos\n", Reason::InvalidCharacter { byte_index: 5 }),
        ("todos\0", Reason::InvalidCharacter { byte_index: 5 }),
        ("todos.title", Reason::InvalidCharacter { byte_index: 5 }),
        (
            "todos;DROP_TABLE",
            Reason::InvalidCharacter { byte_index: 5 },
        ),
        ("todos--comment", Reason::InvalidCharacter { byte_index: 5 }),
        (
            "todos/*comment*/",
            Reason::InvalidCharacter { byte_index: 5 },
        ),
        (
            "todos WHERE id=1",
            Reason::InvalidCharacter { byte_index: 5 },
        ),
        ("todos,title", Reason::InvalidCharacter { byte_index: 5 }),
        ("todos\"", Reason::InvalidCharacter { byte_index: 5 }),
        ("todos'", Reason::InvalidCharacter { byte_index: 5 }),
        ("todosé", Reason::InvalidCharacter { byte_index: 5 }),
    ] {
        invalid(Table::new(name), Context::TableIdentifier, reason);
        invalid(Column::new(name), Context::ColumnIdentifier, reason);
    }
}

#[test]
fn comparisons_reject_null_for_every_operator_without_rewriting_it() {
    for operator in [Op::Equal, Op::NotEqual, Op::GreaterThan, Op::LessThan] {
        invalid(
            Predicate::compare(column("completed"), operator, Value::Null),
            Context::Comparison,
            Reason::NullComparison,
        );
    }
}

#[test]
fn logical_composition_preserves_parentheses_child_order_and_ownership() {
    let a = Predicate::compare(column("id"), Op::GreaterThan, Value::Integer(5)).unwrap();
    let b = Predicate::compare(
        column("title"),
        Op::NotEqual,
        Value::Text("private-title".into()),
    )
    .unwrap();
    let c = Predicate::compare(column("id"), Op::LessThan, Value::Integer(100)).unwrap();
    let grouped = a.clone().or(b.clone()).and(c.clone());
    let different = a.clone().or(b.clone().and(c.clone()));
    assert_ne!(grouped, different);

    let Predicate::And(left, right) = grouped else {
        panic!("expected outer AND")
    };
    assert_eq!(*right, c);
    let Predicate::Or(first, second) = *left else {
        panic!("expected grouped OR")
    };
    assert_eq!(*first, a);
    assert_eq!(*second, b);
    let Predicate::Comparison(comparison) = *second else {
        panic!("expected comparison")
    };
    assert_eq!(comparison.column().as_str(), "title");
    assert_eq!(comparison.operator(), Op::NotEqual);
    assert_eq!(comparison.value(), &Value::Text("private-title".into()));
}

#[test]
fn projections_distinguish_all_from_explicit_ordered_columns_and_reject_empty() {
    invalid(
        Projection::columns(vec![]),
        Context::Projection,
        Reason::Empty,
    );
    assert!(Projection::all().as_columns().is_none());
    let projection =
        Projection::columns(vec![column("title"), column("id"), column("title")]).unwrap();
    let names: Vec<_> = projection
        .as_columns()
        .unwrap()
        .iter()
        .map(Column::as_str)
        .collect();
    assert_eq!(names, ["title", "id", "title"]);
    let select = Select::new(table(), projection.clone());
    assert_eq!(select.table().as_str(), "todos");
    assert_eq!(select.projection(), &projection);
    assert!(select.filter().is_none());
    assert!(select.ordering().is_empty());
    assert_eq!((select.limit(), select.offset()), (None, None));
}

#[test]
fn select_preserves_grouped_filter_and_ordering_priority_and_pagination_options() {
    let filter =
        by_id()
            .or(Predicate::compare(column("completed"), Op::Equal, Value::Boolean(false)).unwrap());
    let ordering = vec![
        Ordering {
            column: column("completed"),
            direction: SortDirection::Ascending,
        },
        Ordering {
            column: column("id"),
            direction: SortDirection::Descending,
        },
    ];
    let select = Select::new(table(), Projection::all())
        .with_filter(filter.clone())
        .with_ordering(ordering.clone())
        .with_limit(20)
        .with_offset(40);
    assert_eq!(select.filter(), Some(&filter));
    assert_eq!(select.ordering(), ordering);
    assert_eq!((select.limit(), select.offset()), (Some(20), Some(40)));

    let replaced = select
        .with_filter(by_id())
        .with_ordering(vec![])
        .with_limit(0)
        .with_offset(0);
    assert_eq!(replaced.filter(), Some(&by_id()));
    assert!(replaced.ordering().is_empty());
    assert_eq!((replaced.limit(), replaced.offset()), (Some(0), Some(0)));
    let offset_only = Select::new(table(), Projection::all()).with_offset(u64::MAX);
    assert_eq!(
        (offset_only.limit(), offset_only.offset()),
        (None, Some(u64::MAX))
    );
}

#[test]
fn insert_rejects_empty_and_duplicate_columns_including_case_variants() {
    invalid(Insert::new(table(), vec![]), Context::Insert, Reason::Empty);
    for duplicate in ["title", "TITLE", "Title"] {
        let mut pairs = assignments();
        pairs.push(Assignment {
            column: column(duplicate),
            value: Value::Null,
        });
        invalid(
            Insert::new(table(), pairs),
            Context::Insert,
            Reason::DuplicateColumn { index: 2 },
        );
    }
}

#[test]
fn both_update_scopes_reject_empty_and_duplicate_columns() {
    invalid(
        Update::new(table(), vec![], by_id()),
        Context::Update,
        Reason::Empty,
    );
    invalid(
        Update::all_rows(table(), vec![]),
        Context::Update,
        Reason::Empty,
    );
    for duplicate in ["title", "TITLE"] {
        let mut pairs = assignments();
        pairs.push(Assignment {
            column: column(duplicate),
            value: Value::Null,
        });
        invalid(
            Update::new(table(), pairs.clone(), by_id()),
            Context::Update,
            Reason::DuplicateColumn { index: 2 },
        );
        invalid(
            Update::all_rows(table(), pairs),
            Context::Update,
            Reason::DuplicateColumn { index: 2 },
        );
    }
}

#[test]
fn assignments_keep_column_value_pairs_in_input_order_and_allow_null() {
    let mut pairs = assignments();
    pairs.push(Assignment {
        column: column("optional_note"),
        value: Value::Null,
    });
    let insert = Insert::new(table(), pairs.clone()).unwrap();
    let update = Update::new(table(), pairs, by_id()).unwrap();
    for actual in [insert.assignments(), update.assignments()] {
        let names: Vec<_> = actual.iter().map(|pair| pair.column.as_str()).collect();
        let values: Vec<_> = actual.iter().map(|pair| &pair.value).collect();
        assert_eq!(names, ["title", "completed", "optional_note"]);
        assert_eq!(
            values,
            [
                &Value::Text("private-title".into()),
                &Value::Boolean(false),
                &Value::Null
            ]
        );
    }
    assert_eq!(insert.table().as_str(), "todos");
    assert_eq!(update.table(), insert.table());
}

#[test]
fn mutations_retain_filters_and_require_named_all_rows_constructors_to_omit_them() {
    let filter = by_id()
        .and(Predicate::compare(column("completed"), Op::Equal, Value::Boolean(true)).unwrap());
    let update = Update::new(table(), assignments(), filter.clone()).unwrap();
    let delete = Delete::new(table(), filter.clone());
    assert_eq!(update.filter(), Some(&filter));
    assert_eq!(delete.filter(), Some(&filter));
    let all_update = Update::all_rows(table(), assignments()).unwrap();
    let all_delete = Delete::all_rows(table());
    assert!(all_update.filter().is_none());
    assert!(all_delete.filter().is_none());
    assert_eq!(delete.table(), all_delete.table());
    assert_eq!(update.assignments(), all_update.assignments());
}

#[test]
fn query_debug_redacts_values_at_every_statement_and_predicate_nesting_level() {
    let filter = Predicate::compare(
        column("title"),
        Op::Equal,
        Value::Text("private-filter".into()),
    )
    .unwrap();
    let queries = [
        Query::Select(
            Select::new(table(), Projection::all()).with_filter(filter.clone().or(by_id())),
        ),
        Query::Insert(Insert::new(table(), assignments()).unwrap()),
        Query::Update(Update::new(table(), assignments(), filter.clone()).unwrap()),
        Query::Delete(Delete::new(table(), filter)),
    ];
    for query in queries {
        for output in [format!("{query:?}"), format!("{query:#?}")] {
            for secret in ["private-filter", "private-title", "42", "false"] {
                assert!(!output.contains(secret));
            }
        }
    }
    let error = Table::new("secret-name; DROP TABLE todos").unwrap_err();
    assert!(!format!("{error:?} {error}").contains("secret-name"));
    let OrmError::Validation(details) = error else {
        panic!("expected validation error")
    };
    assert!(!format!("{details:?}").contains("secret-name"));
}
