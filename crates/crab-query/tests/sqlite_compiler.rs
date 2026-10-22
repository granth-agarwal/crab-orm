//! SQL goldens, emitted bind order, quoting, and reachable compiler failures.

use crab_core::{
    Assignment, Column, ComparisonOperator as Op, CompiledQuery, Delete, Insert, Ordering,
    OrmError, Predicate, Projection, Query, Select, SortDirection, Table, Update, Value,
};
use crab_query::{QueryCompiler, SqliteCompiler};

fn table() -> Table {
    Table::new("todos").unwrap()
}
fn column(name: &str) -> Column {
    Column::new(name).unwrap()
}
fn compare(name: &str, op: Op, value: Value) -> Predicate {
    Predicate::compare(column(name), op, value).unwrap()
}
fn pair(name: &str, value: Value) -> Assignment {
    Assignment {
        column: column(name),
        value,
    }
}
fn compile(query: Query) -> CompiledQuery {
    SqliteCompiler.compile(&query).unwrap()
}

fn assert_compiled(query: CompiledQuery, sql: &str, params: Vec<Value>) {
    assert_eq!(query.sql, sql);
    assert_eq!(query.params, params);
    // Validated identifiers cannot contain '?'; every generated placeholder here
    // must have one emitted portable value. This does not parse arbitrary SQL.
    assert_eq!(query.sql.matches('?').count(), query.params.len());
}

#[test]
fn select_golden_preserves_grouping_ordering_and_pagination_bind_order() {
    let filter = compare("completed", Op::Equal, Value::Boolean(false))
        .or(compare(
            "title",
            Op::NotEqual,
            Value::Text("select-private-sentinel".into()),
        ))
        .and(compare("id", Op::GreaterThan, Value::Integer(310_001)))
        .and(compare("id", Op::LessThan, Value::Integer(390_001)));
    let query = Select::new(
        table(),
        Projection::columns(vec![column("id"), column("title"), column("completed")]).unwrap(),
    )
    .with_filter(filter)
    .with_ordering(vec![
        Ordering {
            column: column("completed"),
            direction: SortDirection::Ascending,
        },
        Ordering {
            column: column("id"),
            direction: SortDirection::Descending,
        },
    ])
    .with_limit(7)
    .with_offset(11);
    assert_compiled(
        compile(Query::Select(query)),
        r#"SELECT "id", "title", "completed" FROM "todos" WHERE ("completed" = ? OR "title" <> ?) AND "id" > ? AND "id" < ? ORDER BY "completed" ASC, "id" DESC LIMIT ? OFFSET ?"#,
        vec![
            Value::Boolean(false),
            Value::Text("select-private-sentinel".into()),
            Value::Integer(310_001),
            Value::Integer(390_001),
            Value::Integer(7),
            Value::Integer(11),
        ],
    );
}

#[test]
fn insert_golden_keeps_all_portable_value_types_bound_in_assignment_order() {
    let values = vec![
        Value::Text("O'Reilly'); DROP TABLE todos; -- insert-private-sentinel".into()),
        Value::Boolean(true),
        Value::Real(98765.4321),
        Value::Blob(b"blob-private-sentinel\0\xff".to_vec()),
        Value::Null,
        Value::Integer(i64::MIN),
    ];
    let assignments = ["title", "completed", "estimate", "payload", "note", "id"]
        .into_iter()
        .zip(values.clone())
        .map(|(name, value)| pair(name, value))
        .collect();
    let query = compile(Query::Insert(Insert::new(table(), assignments).unwrap()));
    for sentinel in [
        "insert-private-sentinel",
        "DROP TABLE",
        "O'Reilly",
        "98765.4321",
        "blob-private-sentinel",
    ] {
        assert!(!query.sql.contains(sentinel));
    }
    assert_compiled(
        query,
        r#"INSERT INTO "todos" ("title", "completed", "estimate", "payload", "note", "id") VALUES (?, ?, ?, ?, ?, ?)"#,
        values,
    );
}

#[test]
fn update_golden_emits_assignment_binds_before_nested_predicate_binds() {
    let filter = compare("id", Op::Equal, Value::Integer(42_001)).and(
        compare(
            "title",
            Op::NotEqual,
            Value::Text("predicate-private-sentinel".into()),
        )
        .or(compare("completed", Op::Equal, Value::Boolean(false))),
    );
    let assignments = vec![
        pair("title", Value::Text("update-private-sentinel".into())),
        pair("completed", Value::Boolean(true)),
        pair("estimate", Value::Real(-123.75)),
        pair("note", Value::Null),
    ];
    assert_compiled(
        compile(Query::Update(
            Update::new(table(), assignments, filter).unwrap(),
        )),
        r#"UPDATE "todos" SET "title" = ?, "completed" = ?, "estimate" = ?, "note" = ? WHERE "id" = ? AND ("title" <> ? OR "completed" = ?)"#,
        vec![
            Value::Text("update-private-sentinel".into()),
            Value::Boolean(true),
            Value::Real(-123.75),
            Value::Null,
            Value::Integer(42_001),
            Value::Text("predicate-private-sentinel".into()),
            Value::Boolean(false),
        ],
    );
}

#[test]
fn delete_golden_preserves_or_with_nested_and() {
    let filter = compare("id", Op::LessThan, Value::Integer(17_001)).or(compare(
        "completed",
        Op::Equal,
        Value::Boolean(false),
    )
    .and(compare("id", Op::GreaterThan, Value::Integer(88_001))));
    assert_compiled(
        compile(Query::Delete(Delete::new(table(), filter))),
        r#"DELETE FROM "todos" WHERE "id" < ? OR ("completed" = ? AND "id" > ?)"#,
        vec![
            Value::Integer(17_001),
            Value::Boolean(false),
            Value::Integer(88_001),
        ],
    );
}

#[test]
fn offset_without_limit_uses_bound_unbounded_marker_and_keeps_input_unchanged() {
    let select = Select::new(table(), Projection::all())
        .with_filter(compare("id", Op::GreaterThan, Value::Integer(500_001)))
        .with_offset(13);
    let query = Query::Select(select);
    let compiled = SqliteCompiler.compile(&query).unwrap();
    assert_compiled(
        compiled,
        r#"SELECT * FROM "todos" WHERE "id" > ? LIMIT ? OFFSET ?"#,
        vec![
            Value::Integer(500_001),
            Value::Integer(-1),
            Value::Integer(13),
        ],
    );
    let Query::Select(select) = query else {
        panic!("expected SELECT")
    };
    assert_eq!(select.limit(), None);
    assert_eq!(select.offset(), Some(13));
}

#[test]
fn pagination_distinguishes_unspecified_zero_and_maximum_signed_values() {
    assert_compiled(
        compile(Query::Select(Select::new(table(), Projection::all()))),
        r#"SELECT * FROM "todos""#,
        vec![],
    );
    assert_compiled(
        compile(Query::Select(
            Select::new(table(), Projection::all()).with_limit(0),
        )),
        r#"SELECT * FROM "todos" LIMIT ?"#,
        vec![Value::Integer(0)],
    );
    assert_compiled(
        compile(Query::Select(
            Select::new(table(), Projection::all()).with_offset(0),
        )),
        r#"SELECT * FROM "todos" LIMIT ? OFFSET ?"#,
        vec![Value::Integer(-1), Value::Integer(0)],
    );
    assert_compiled(
        compile(Query::Select(
            Select::new(table(), Projection::all())
                .with_limit(0)
                .with_offset(0),
        )),
        r#"SELECT * FROM "todos" LIMIT ? OFFSET ?"#,
        vec![Value::Integer(0), Value::Integer(0)],
    );
    assert_compiled(
        compile(Query::Select(
            Select::new(table(), Projection::all())
                .with_limit(i64::MAX as u64)
                .with_offset(i64::MAX as u64),
        )),
        r#"SELECT * FROM "todos" LIMIT ? OFFSET ?"#,
        vec![Value::Integer(i64::MAX), Value::Integer(i64::MAX)],
    );
}

#[test]
fn unsigned_pagination_overflow_is_a_compile_error_instead_of_truncation() {
    for value in [(i64::MAX as u64) + 1, u64::MAX] {
        for select in [
            Select::new(table(), Projection::all()).with_limit(value),
            Select::new(table(), Projection::all()).with_offset(value),
        ] {
            let error = SqliteCompiler.compile(&Query::Select(select)).unwrap_err();
            assert!(matches!(&error, OrmError::Compile(message) if message.contains("i64 range")));
            assert!(!format!("{error} {error:?}").contains(&value.to_string()));
        }
    }
}

#[test]
fn explicitly_unfiltered_mutations_have_no_implicit_predicates() {
    assert_compiled(
        compile(Query::Update(
            Update::all_rows(table(), vec![pair("completed", Value::Boolean(false))]).unwrap(),
        )),
        r#"UPDATE "todos" SET "completed" = ?"#,
        vec![Value::Boolean(false)],
    );
    assert_compiled(
        compile(Query::Delete(Delete::all_rows(table()))),
        r#"DELETE FROM "todos""#,
        vec![],
    );
}

#[test]
fn keyword_identifiers_are_quoted_and_projection_order_is_retained() {
    let query = Select::new(
        Table::new("order").unwrap(),
        Projection::columns(vec![column("select"), column("group"), column("select")]).unwrap(),
    )
    .with_filter(compare(
        "select",
        Op::Equal,
        Value::Text("keyword-private-sentinel".into()),
    ))
    .with_ordering(vec![Ordering {
        column: column("group"),
        direction: SortDirection::Descending,
    }]);
    assert_compiled(
        compile(Query::Select(query)),
        r#"SELECT "select", "group", "select" FROM "order" WHERE "select" = ? ORDER BY "group" DESC"#,
        vec![Value::Text("keyword-private-sentinel".into())],
    );
}

#[test]
fn quote_and_injection_shaped_text_stays_bound_in_every_statement() {
    let payload = "PRIVATE_b9e12: O'Reilly ?'); DROP TABLE todos; --";
    let value = Value::Text(payload.into());
    let predicate = compare("title", Op::Equal, value.clone());
    let queries = [
        Query::Select(Select::new(table(), Projection::all()).with_filter(predicate.clone())),
        Query::Insert(Insert::new(table(), vec![pair("title", value.clone())]).unwrap()),
        Query::Update(
            Update::new(
                table(),
                vec![pair("title", value.clone())],
                predicate.clone(),
            )
            .unwrap(),
        ),
        Query::Delete(Delete::new(table(), predicate)),
    ];
    for query in queries {
        let first = SqliteCompiler.compile(&query).unwrap();
        let second = SqliteCompiler.compile(&query).unwrap();
        assert_eq!(first, second);
        for sentinel in [payload, "PRIVATE_b9e12", "O'Reilly", "DROP TABLE"] {
            assert!(!first.sql.contains(sentinel));
        }
        assert!(first.params.iter().all(|param| param == &value));
        assert!(!first.params.is_empty());
        assert_eq!(first.sql.matches('?').count(), first.params.len());
        assert!(!format!("{first} {first:?}").contains(payload));
    }
}

#[test]
fn null_and_empty_text_or_blob_remain_distinct_bound_values() {
    let values = vec![
        Value::Null,
        Value::Text(String::new()),
        Value::Blob(vec![]),
        Value::Integer(0),
        Value::Boolean(false),
        Value::Real(-0.0),
    ];
    let assignments = ["note", "title", "payload", "id", "completed", "estimate"]
        .into_iter()
        .zip(values.clone())
        .map(|(name, value)| pair(name, value))
        .collect();
    let query = compile(Query::Insert(Insert::new(table(), assignments).unwrap()));
    assert_compiled(
        query,
        r#"INSERT INTO "todos" ("note", "title", "payload", "id", "completed", "estimate") VALUES (?, ?, ?, ?, ?, ?)"#,
        values,
    );
    // NULL comparisons are rejected before compilation; no NULL test IR exists.
    assert!(matches!(
        Predicate::compare(column("id"), Op::Equal, Value::Null),
        Err(OrmError::Validation(_))
    ));
}

#[test]
fn non_finite_real_values_fail_in_assignments_and_predicates_for_all_statements() {
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let predicate = compare("estimate", Op::GreaterThan, Value::Real(value));
        let queries = [
            Query::Select(Select::new(table(), Projection::all()).with_filter(predicate.clone())),
            Query::Insert(
                Insert::new(table(), vec![pair("estimate", Value::Real(value))]).unwrap(),
            ),
            Query::Update(
                Update::all_rows(table(), vec![pair("estimate", Value::Real(value))]).unwrap(),
            ),
            Query::Delete(Delete::new(table(), predicate)),
        ];
        for query in queries {
            let error = SqliteCompiler.compile(&query).unwrap_err();
            assert!(
                matches!(error, OrmError::Compile(message) if message.contains("non-finite Real"))
            );
        }
    }
}
