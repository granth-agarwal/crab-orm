//! SQLite adapter integration tests using isolated connections and temporary files.
#![cfg(feature = "sqlite")]

use crab_core::{CompiledQuery, ConstraintKind, DriverError, OrmError, Value};
use crab_driver::{DatabaseExecutor, Row, SqliteConfig, SqliteExecutor};

fn query(sql: &str, params: Vec<Value>) -> CompiledQuery {
    CompiledQuery {
        sql: sql.into(),
        params,
    }
}

async fn memory() -> SqliteExecutor {
    SqliteExecutor::connect(SqliteConfig::in_memory())
        .await
        .unwrap()
}

async fn execute(db: &mut SqliteExecutor, sql: &str, params: Vec<Value>) -> u64 {
    db.execute(query(sql, params)).await.unwrap().rows_affected
}

fn values(row: Row) -> Vec<Value> {
    row.columns.into_iter().map(|column| column.value).collect()
}

fn error<T: std::fmt::Debug>(result: Result<T, OrmError>, expected: DriverError) {
    assert_eq!(result.unwrap_err(), OrmError::Driver(expected));
}

#[tokio::test(flavor = "current_thread")]
async fn memory_connections_are_isolated_and_explicitly_closed() {
    let mut first = memory().await;
    let mut second = memory().await;
    assert_eq!(
        execute(&mut first, "CREATE TABLE todos (id INTEGER)", vec![]).await,
        0
    );
    execute(
        &mut first,
        "INSERT INTO todos VALUES (?)",
        vec![Value::Integer(7)],
    )
    .await;
    assert_eq!(
        values(
            first
                .fetch_one(query("SELECT id FROM todos", vec![]))
                .await
                .unwrap()
        ),
        [Value::Integer(7)]
    );
    error(
        second
            .fetch_all(query("SELECT id FROM todos", vec![]))
            .await,
        DriverError::Execution,
    );
    first.close().await.unwrap();
    second.close().await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn file_creation_is_explicit_and_data_survives_close_and_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("todos.sqlite");
    let config = SqliteConfig::file(&path).unwrap();
    error(
        SqliteExecutor::connect(config.clone()).await,
        DriverError::Connection,
    );
    assert!(!path.exists());
    let mut db = SqliteExecutor::connect(config.clone().create_if_missing(true))
        .await
        .unwrap();
    execute(&mut db, "CREATE TABLE todos (title TEXT)", vec![]).await;
    execute(
        &mut db,
        "INSERT INTO todos VALUES (?)",
        vec![Value::Text("persistent".into())],
    )
    .await;
    db.close().await.unwrap();
    assert!(path.exists());
    let mut reopened = SqliteExecutor::connect(config).await.unwrap();
    assert_eq!(
        values(
            reopened
                .fetch_one(query("SELECT title FROM todos", vec![]))
                .await
                .unwrap()
        ),
        [Value::Text("persistent".into())]
    );
    reopened.close().await.unwrap();
    directory.close().unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn urls_are_fallible_redacted_and_respect_explicit_creation_mode() {
    for url in [
        "",
        "postgres://private-password",
        "sqlite:",
        "sqlite::memory:?mode=invalid",
        "sqlite:bad%00path",
    ] {
        error(SqliteConfig::from_url(url), DriverError::Configuration);
    }
    error(SqliteConfig::file(""), DriverError::Configuration);
    error(SqliteConfig::file(":memory:"), DriverError::Configuration);
    let config = SqliteConfig::from_url("sqlite::memory:").unwrap();
    assert!(!format!("{config:?}").contains(":memory:"));
    let mut db = SqliteExecutor::connect(config).await.unwrap();
    assert_eq!(
        values(db.fetch_one(query("SELECT 17", vec![])).await.unwrap()),
        [Value::Integer(17)]
    );
    db.close().await.unwrap();
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("from-url.sqlite");
    let url = format!("sqlite://{}?mode=rwc", path.display());
    let config = SqliteConfig::from_url(&url).unwrap();
    assert!(!format!("{config:?}").contains(directory.path().to_str().unwrap()));
    SqliteExecutor::connect(config)
        .await
        .unwrap()
        .close()
        .await
        .unwrap();
    assert!(path.exists());
    let inaccessible = directory
        .path()
        .join("missing-directory")
        .join("never.sqlite");
    error(
        SqliteExecutor::connect(
            SqliteConfig::file(&inaccessible)
                .unwrap()
                .create_if_missing(true),
        )
        .await,
        DriverError::Connection,
    );
    assert!(!inaccessible.exists());
    directory.close().unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn fetch_one_returns_first_or_not_found_and_fetch_all_preserves_all_rows() {
    let mut db = memory().await;
    execute(&mut db, "CREATE TABLE numbers (n INTEGER)", vec![]).await;
    assert!(
        db.fetch_all(query("SELECT n FROM numbers", vec![]))
            .await
            .unwrap()
            .is_empty()
    );
    error(
        db.fetch_one(query("SELECT n FROM numbers", vec![])).await,
        DriverError::NotFound,
    );
    assert_eq!(
        execute(
            &mut db,
            "INSERT INTO numbers VALUES (?), (?)",
            vec![Value::Integer(2), Value::Integer(1)]
        )
        .await,
        2
    );
    let sql = "SELECT n FROM numbers ORDER BY n";
    assert_eq!(
        values(db.fetch_one(query(sql, vec![])).await.unwrap()),
        [Value::Integer(1)]
    );
    let rows = db.fetch_all(query(sql, vec![])).await.unwrap();
    assert_eq!(
        rows.into_iter().map(values).collect::<Vec<_>>(),
        [vec![Value::Integer(1)], vec![Value::Integer(2)]]
    );
    assert_eq!(
        execute(&mut db, "UPDATE numbers SET n = n + 10", vec![]).await,
        2
    );
    // DDL must not reuse SQLite's preceding nonzero DML changes() count.
    assert_eq!(
        execute(&mut db, "CREATE TABLE another (n INTEGER)", vec![]).await,
        0
    );
    assert_eq!(
        execute(
            &mut db,
            "DELETE FROM numbers WHERE n > ?",
            vec![Value::Integer(100)]
        )
        .await,
        0
    );
    db.close().await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn binds_round_trip_without_interpolation_and_booleans_read_as_integers() {
    let mut db = memory().await;
    let payload = "PRIVATE_71: O'Reilly ?'); DROP TABLE preserved; --\0";
    execute(&mut db, "CREATE TABLE preserved (id INTEGER)", vec![]).await;
    let params = vec![
        Value::Null,
        Value::Integer(i64::MIN),
        Value::Integer(i64::MAX),
        Value::Real(-19.125),
        Value::Text(payload.into()),
        Value::Blob(vec![0, 255, 17]),
        Value::Boolean(true),
        Value::Boolean(false),
        Value::Text(String::new()),
        Value::Blob(vec![]),
    ];
    let row = db.fetch_one(query("SELECT ? AS n, ? AS lo, ? AS hi, ? AS r, ? AS t, ? AS b, ? AS yes, ? AS no, ? AS empty_text, ? AS empty_blob", params.clone())).await.unwrap();
    assert_eq!(
        row.columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        [
            "n",
            "lo",
            "hi",
            "r",
            "t",
            "b",
            "yes",
            "no",
            "empty_text",
            "empty_blob"
        ]
    );
    let mut expected = params;
    expected[6] = Value::Integer(1);
    expected[7] = Value::Integer(0);
    assert_eq!(values(row), expected);
    assert!(
        db.fetch_all(query("SELECT * FROM preserved", vec![]))
            .await
            .unwrap()
            .is_empty()
    );
    db.close().await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn dynamically_typed_cells_expressions_and_duplicate_labels_preserve_runtime_values() {
    let mut db = memory().await;
    execute(&mut db, "CREATE TABLE mixed (value BOOLEAN)", vec![]).await;
    let expected = vec![
        Value::Integer(7),
        Value::Real(1.25),
        Value::Text("not-a-boolean".into()),
        Value::Blob(vec![9, 0]),
        Value::Null,
    ];
    for value in &expected {
        execute(&mut db, "INSERT INTO mixed VALUES (?)", vec![value.clone()]).await;
    }
    let rows = db
        .fetch_all(query("SELECT value FROM mixed ORDER BY rowid", vec![]))
        .await
        .unwrap();
    assert_eq!(
        rows.into_iter()
            .map(|row| values(row).remove(0))
            .collect::<Vec<_>>(),
        expected
    );
    let row = db.fetch_one(query("SELECT 1 + 2 AS duplicate, 'x' || 'y' AS duplicate, CAST(NULL AS DATE) AS missing, x'00FF' AS bytes", vec![])).await.unwrap();
    assert_eq!(
        row.columns
            .iter()
            .map(|column| column.name.as_str())
            .collect::<Vec<_>>(),
        ["duplicate", "duplicate", "missing", "bytes"]
    );
    assert_eq!(
        values(row),
        [
            Value::Integer(3),
            Value::Text("xy".into()),
            Value::Null,
            Value::Blob(vec![0, 255])
        ]
    );
    db.close().await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn invalid_utf8_text_is_a_row_conversion_error_not_lossy_text() {
    let mut db = memory().await;
    error(
        db.fetch_one(query("SELECT CAST(x'80' AS TEXT)", vec![]))
            .await,
        DriverError::RowConversion { column: 0 },
    );
    db.close().await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn constraints_are_structured_and_foreign_keys_are_enabled() {
    let mut db = memory().await;
    assert_eq!(
        values(
            db.fetch_one(query("PRAGMA foreign_keys", vec![]))
                .await
                .unwrap()
        ),
        [Value::Integer(1)]
    );
    execute(&mut db, "CREATE TABLE parent (id INTEGER PRIMARY KEY, title TEXT NOT NULL UNIQUE, score INTEGER CHECK(score > 0))", vec![]).await;
    execute(
        &mut db,
        "CREATE TABLE child (parent_id INTEGER REFERENCES parent(id))",
        vec![],
    )
    .await;
    execute(
        &mut db,
        "INSERT INTO parent VALUES (?, ?, ?)",
        vec![
            Value::Integer(1),
            Value::Text("private-constraint-value".into()),
            Value::Integer(2),
        ],
    )
    .await;
    for (sql, params, kind) in [
        (
            "INSERT INTO parent VALUES (2, ?, 1)",
            vec![Value::Text("private-constraint-value".into())],
            ConstraintKind::Unique,
        ),
        (
            "INSERT INTO parent VALUES (3, ?, 1)",
            vec![Value::Null],
            ConstraintKind::NotNull,
        ),
        (
            "INSERT INTO parent VALUES (4, 'check', ?)",
            vec![Value::Integer(-1)],
            ConstraintKind::Check,
        ),
        (
            "INSERT INTO child VALUES (?)",
            vec![Value::Integer(999)],
            ConstraintKind::ForeignKey,
        ),
    ] {
        let result = db.execute(query(sql, params)).await;
        assert!(!format!("{result:?}").contains("private-constraint-value"));
        error(result, DriverError::Constraint(kind));
    }
    db.close().await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn missing_extra_and_unsupported_placeholders_fail_before_dml_executes() {
    let mut db = memory().await;
    execute(&mut db, "CREATE TABLE data (value)", vec![]).await;
    error(
        db.execute(query("INSERT INTO data VALUES (?)", vec![]))
            .await,
        DriverError::ParameterCount {
            expected: 1,
            actual: 0,
        },
    );
    error(
        db.execute(query(
            "INSERT INTO data VALUES (?)",
            vec![Value::Integer(1), Value::Integer(2)],
        ))
        .await,
        DriverError::ParameterCount {
            expected: 1,
            actual: 2,
        },
    );
    error(
        db.fetch_one(query("SELECT ?", vec![])).await,
        DriverError::ParameterCount {
            expected: 1,
            actual: 0,
        },
    );
    error(
        db.fetch_all(query("SELECT 1", vec![Value::Integer(1)]))
            .await,
        DriverError::ParameterCount {
            expected: 0,
            actual: 1,
        },
    );
    for sql in [
        "INSERT INTO data VALUES (?1)",
        "INSERT INTO data VALUES (?2)",
        "INSERT INTO data VALUES (:value)",
        "INSERT INTO data VALUES (@value)",
        "INSERT INTO data VALUES ($1)",
    ] {
        error(
            db.execute(query(sql, vec![Value::Integer(1)])).await,
            DriverError::UnsupportedQuery,
        );
    }
    assert!(
        db.fetch_all(query("SELECT value FROM data", vec![]))
            .await
            .unwrap()
            .is_empty()
    );
    db.close().await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn single_statement_guard_handles_literals_comments_and_rejects_batches() {
    let mut db = memory().await;
    execute(&mut db, "CREATE TABLE data (value)", vec![]).await;
    error(
        db.execute(query(
            "INSERT INTO data VALUES (?); INSERT INTO data VALUES (?)",
            vec![Value::Integer(1), Value::Integer(2)],
        ))
        .await,
        DriverError::UnsupportedQuery,
    );
    assert!(
        db.fetch_all(query("SELECT value FROM data", vec![]))
            .await
            .unwrap()
            .is_empty()
    );
    let row = db.fetch_one(query("/* ?; :x */ SELECT '?;''quote' AS \"double\"\"?;\", ? AS [bracket?;], 'doubled' AS `ticks``?;` ; -- ?; trailing", vec![Value::Text("bound".into())])).await.unwrap();
    assert_eq!(
        values(row),
        [
            Value::Text("?;'quote".into()),
            Value::Text("bound".into()),
            Value::Text("doubled".into())
        ]
    );
    for sql in [
        "",
        " -- only a comment",
        "/* only a comment */",
        ";",
        "SELECT 1\0;SELECT 2",
        "SELECT 'unterminated",
        "SELECT 1 /* unterminated",
    ] {
        error(
            db.fetch_all(query(sql, vec![])).await,
            DriverError::InvalidQuery,
        );
    }
    error(
        db.fetch_one(query("SELEC invalid", vec![])).await,
        DriverError::Execution,
    );
    db.close().await.unwrap();
}

#[tokio::test(flavor = "current_thread")]
async fn method_result_shapes_and_non_finite_parameters_are_rejected() {
    let mut db = memory().await;
    execute(&mut db, "CREATE TABLE data (value)", vec![]).await;
    error(
        db.execute(query("SELECT 1", vec![])).await,
        DriverError::UnsupportedQuery,
    );
    error(
        db.fetch_all(query("INSERT INTO data VALUES (1)", vec![]))
            .await,
        DriverError::UnsupportedQuery,
    );
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        error(
            db.execute(query(
                "INSERT INTO data VALUES (?)",
                vec![Value::Real(value)],
            ))
            .await,
            DriverError::InvalidParameter { index: 0 },
        );
    }
    assert!(
        db.fetch_all(query("SELECT value FROM data", vec![]))
            .await
            .unwrap()
            .is_empty()
    );
    db.close().await.unwrap();
}
