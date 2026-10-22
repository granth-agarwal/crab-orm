//! Compile and execute owned queries through the facade on private SQLite memory.

use crab_orm::{
    Assignment, Column, ComparisonOperator as Op, CompiledQuery, DatabaseExecutor, Insert,
    OrmError, Predicate, Projection, Query, QueryCompiler, Select, SqliteCompiler, SqliteConfig,
    SqliteExecutor, Table, Value,
};

fn inspect(query: &CompiledQuery) {
    // These generated statements contain placeholders, not sensitive literals.
    println!("SQL: {}", query.sql);
    println!("Parameter count: {} (values redacted)", query.params.len());
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), OrmError> {
    let mut db = SqliteExecutor::connect(SqliteConfig::in_memory()).await?;
    db.execute(CompiledQuery {
        sql: "CREATE TABLE todos (id INTEGER PRIMARY KEY, title TEXT NOT NULL, completed INTEGER NOT NULL)".into(),
        params: vec![],
    })
    .await?;

    let todos = Table::new("todos")?;
    let id = Column::new("id")?;
    let title = Column::new("title")?;
    let completed = Column::new("completed")?;
    let insert = Query::Insert(Insert::new(
        todos.clone(),
        vec![
            Assignment {
                column: id.clone(),
                value: Value::Integer(1),
            },
            Assignment {
                column: title.clone(),
                value: Value::Text("Read the docs".into()),
            },
            Assignment {
                column: completed.clone(),
                value: Value::Boolean(false),
            },
        ],
    )?);
    let compiled = SqliteCompiler.compile(&insert)?;
    inspect(&compiled);
    let result = db.execute(compiled).await?;
    println!("Inserted rows: {}", result.rows_affected);

    let select = Query::Select(
        Select::new(
            todos,
            Projection::columns(vec![id.clone(), title, completed])?,
        )
        .with_filter(Predicate::compare(id, Op::Equal, Value::Integer(1))?),
    );
    let compiled = SqliteCompiler.compile(&select)?;
    inspect(&compiled);
    let row = db.fetch_one(compiled).await?;
    // Only this example's fixed, non-sensitive fixture data is inspected.
    for column in row.columns {
        match column.value {
            Value::Integer(value) => println!("{}: {value}", column.name),
            Value::Text(value) => println!("{}: {value}", column.name),
            _ => println!("{}: [redacted]", column.name),
        }
    }
    db.close().await?;
    println!("In-memory SQLite connection closed. Boolean values read back as integers.");
    println!("Model-level CRUD helpers and transaction APIs remain future subphases.");
    Ok(())
}
