//! Real compiler-to-driver integration through CrabORM's public facade.
#![cfg(feature = "sqlite")]

use crab_orm::{
    Assignment, Column, ComparisonOperator as Op, CompiledQuery, DatabaseExecutor, Delete, Insert,
    Ordering, OrmError, Predicate, Projection, Query, QueryCompiler, Select, SortDirection,
    SqliteCompiler, SqliteConfig, SqliteExecutor, Table, Update, Value,
};

#[tokio::test(flavor = "current_thread")]
async fn compiler_output_executes_all_four_statement_types_through_the_facade()
-> Result<(), OrmError> {
    let mut db = SqliteExecutor::connect(SqliteConfig::in_memory()).await?;
    db.execute(CompiledQuery {
        sql: "CREATE TABLE todos (id INTEGER PRIMARY KEY, title TEXT NOT NULL, completed INTEGER NOT NULL)".into(),
        params: vec![],
    }).await?;
    let table = Table::new("todos")?;
    let id = Column::new("id")?;
    let title = Column::new("title")?;
    let completed = Column::new("completed")?;
    let payload = "pipeline sentinel '; DROP TABLE todos; --";
    for index in 1..=3 {
        let insert = Query::Insert(Insert::new(
            table.clone(),
            vec![
                Assignment {
                    column: id.clone(),
                    value: Value::Integer(index),
                },
                Assignment {
                    column: title.clone(),
                    value: Value::Text(payload.into()),
                },
                Assignment {
                    column: completed.clone(),
                    value: Value::Boolean(false),
                },
            ],
        )?);
        let compiled = SqliteCompiler.compile(&insert)?;
        assert!(!compiled.sql.contains(payload));
        assert_eq!(db.execute(compiled).await?.rows_affected, 1);
    }
    let by_id = |value| Predicate::compare(id.clone(), Op::Equal, Value::Integer(value));
    let update = Query::Update(Update::new(
        table.clone(),
        vec![Assignment {
            column: completed.clone(),
            value: Value::Boolean(true),
        }],
        by_id(2)?,
    )?);
    assert_eq!(
        db.execute(SqliteCompiler.compile(&update)?)
            .await?
            .rows_affected,
        1
    );

    let select = Query::Select(
        Select::new(
            table.clone(),
            Projection::columns(vec![id.clone(), title.clone(), completed.clone()])?,
        )
        .with_filter(
            Predicate::compare(completed.clone(), Op::Equal, Value::Boolean(true))?.or(by_id(3)?),
        )
        .with_ordering(vec![Ordering {
            column: id.clone(),
            direction: SortDirection::Ascending,
        }])
        .with_offset(1),
    );
    let rows = db.fetch_all(SqliteCompiler.compile(&select)?).await?;
    assert_eq!(rows.len(), 1);
    assert_eq!(
        rows[0]
            .columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        vec!["id", "title", "completed"]
    );
    assert_eq!(rows[0].columns[0].value, Value::Integer(3));
    assert_eq!(rows[0].columns[1].value, Value::Text(payload.into()));
    assert_eq!(rows[0].columns[2].value, Value::Integer(0));

    let delete = Query::Delete(Delete::new(table.clone(), by_id(2)?));
    assert_eq!(
        db.execute(SqliteCompiler.compile(&delete)?)
            .await?
            .rows_affected,
        1
    );
    let update_all = Query::Update(Update::all_rows(
        table.clone(),
        vec![Assignment {
            column: completed,
            value: Value::Boolean(true),
        }],
    )?);
    assert_eq!(
        db.execute(SqliteCompiler.compile(&update_all)?)
            .await?
            .rows_affected,
        2
    );
    let delete_all = Query::Delete(Delete::all_rows(table.clone()));
    assert_eq!(
        db.execute(SqliteCompiler.compile(&delete_all)?)
            .await?
            .rows_affected,
        2
    );
    let empty = Query::Select(Select::new(table, Projection::all()));
    assert!(
        db.fetch_all(SqliteCompiler.compile(&empty)?)
            .await?
            .is_empty()
    );
    db.close().await
}
