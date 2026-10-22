//! Compile owned query types through the facade; no database I/O.

use crab_orm::{
    Assignment, Column, ComparisonOperator as Op, Delete, Insert, Ordering, OrmError, Predicate,
    Projection, Query, QueryCompiler, Select, SortDirection, SqliteCompiler, Table, Update, Value,
};

fn main() -> Result<(), OrmError> {
    let todos = Table::new("todos")?;
    let id = Column::new("id")?;
    let title = Column::new("title")?;
    let completed = Column::new("completed")?;

    // (completed = false OR title = a value) AND id > a value.
    let filter = Predicate::compare(completed.clone(), Op::Equal, Value::Boolean(false))?
        .or(Predicate::compare(
            title.clone(),
            Op::Equal,
            Value::Text("Read the docs".into()),
        )?)
        .and(Predicate::compare(
            id.clone(),
            Op::GreaterThan,
            Value::Integer(0),
        )?);
    let select = Select::new(
        todos.clone(),
        Projection::columns(vec![id.clone(), title.clone(), completed.clone()])?,
    )
    .with_filter(filter)
    .with_ordering(vec![Ordering {
        column: id.clone(),
        direction: SortDirection::Ascending,
    }])
    .with_limit(10)
    .with_offset(0);

    let insert = Insert::new(
        todos.clone(),
        vec![
            Assignment {
                column: title,
                value: Value::Text("Read the docs".into()),
            },
            Assignment {
                column: completed.clone(),
                value: Value::Boolean(false),
            },
        ],
    )?;
    let by_id = Predicate::compare(id, Op::Equal, Value::Integer(1))?;
    let update = Update::new(
        todos.clone(),
        vec![Assignment {
            column: completed,
            value: Value::Boolean(true),
        }],
        by_id.clone(),
    )?;
    let delete = Delete::new(todos, by_id);

    let queries = [
        Query::Select(select),
        Query::Insert(insert),
        Query::Update(update),
        Query::Delete(delete),
    ];
    println!(
        "CrabORM Phase 1.2: compiling {} owned queries to SQLite SQL through the facade.",
        queries.len()
    );
    for query in queries {
        let compiled = SqliteCompiler.compile(&query)?;
        // Generated SQL contains placeholders; values are deliberately not shown.
        println!("SQL: {}", compiled.sql);
        println!(
            "Parameter count: {} (values redacted)",
            compiled.params.len()
        );
    }
    println!("Compilation only. No SQLite connection was opened or database contacted.");
    println!("Database execution and CRUD persistence are not implemented yet.");
    Ok(())
}
