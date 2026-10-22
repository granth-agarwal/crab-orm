mod input;

use std::{fmt, path::Path};

use crab_core::{CompiledQuery, ConstraintKind, DriverError, OrmError, Value};
use sqlx::{
    AssertSqlSafe, Column as _, ConnectOptions, Connection, Either, Executor, Row as _, SqlSafeStr,
    Statement, TypeInfo, ValueRef,
    sqlite::{SqliteArguments, SqliteConnectOptions, SqliteConnection, SqliteRow, SqliteStatement},
};

use crate::{ColumnValue, DatabaseExecutor, ExecutionResult, Row};

/// Owned SQLite configuration with private driver-specific options.
///
/// File creation defaults to false; URL `mode=rwc` or create_if_missing(true)
/// requests it explicitly. Foreign keys are always enabled on connect and SQLx
/// statement logging is disabled. Debug never reveals a URL or filename.
#[derive(Clone)]
pub struct SqliteConfig {
    options: SqliteConnectOptions,
}

impl fmt::Debug for SqliteConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("SqliteConfig([redacted])")
    }
}

impl SqliteConfig {
    /// Configure a private in-memory database owned by its connection.
    pub fn in_memory() -> Self {
        Self {
            options: options(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .in_memory(true)
                    .shared_cache(false),
            ),
        }
    }

    /// Parse a SQLite URL using SQLx, without changing the process environment.
    ///
    /// Requires a sqlite: scheme and non-empty filename (or :memory:).
    pub fn from_url(url: &str) -> Result<Self, OrmError> {
        if !url.starts_with("sqlite:") || url.as_bytes().contains(&0) {
            return Err(OrmError::Driver(DriverError::Configuration));
        }
        let parsed: SqliteConnectOptions = url
            .parse()
            .map_err(|_| OrmError::Driver(DriverError::Configuration))?;
        valid_path(parsed.get_filename())?;
        Ok(Self {
            options: options(parsed),
        })
    }

    /// Configure a literal filesystem path without constructing a connection URL.
    ///
    /// The file must already exist unless creation is requested separately.
    pub fn file(path: impl AsRef<Path>) -> Result<Self, OrmError> {
        let path = path.as_ref();
        valid_path(path)?;
        if path == Path::new(":memory:") {
            return Err(OrmError::Driver(DriverError::Configuration));
        }
        Ok(Self {
            options: options(SqliteConnectOptions::new().filename(path)),
        })
    }

    /// Explicitly choose whether a missing file may be created; does not create directories.
    pub fn create_if_missing(mut self, create: bool) -> Self {
        self.options = self.options.create_if_missing(create);
        self
    }
}

fn valid_path(path: &Path) -> Result<(), OrmError> {
    if path.as_os_str().is_empty() || path.as_os_str().as_encoded_bytes().contains(&0) {
        return Err(OrmError::Driver(DriverError::Configuration));
    }
    Ok(())
}

fn options(options: SqliteConnectOptions) -> SqliteConnectOptions {
    options.foreign_keys(true).disable_statement_logging()
}

/// A single owned SQLite connection, available only with the sqlite feature.
///
/// Methods exclusively borrow the connection and return Send futures. close(self)
/// consumes it and awaits SQLx worker shutdown; Drop initiates cleanup but cannot
/// promise synchronous completion. No connection pool or transaction API exists.
pub struct SqliteExecutor {
    connection: SqliteConnection,
}

impl fmt::Debug for SqliteExecutor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SqliteExecutor").finish_non_exhaustive()
    }
}

impl SqliteExecutor {
    /// Open the configured database. File creation and FK enforcement use SQLx options.
    pub async fn connect(config: SqliteConfig) -> Result<Self, OrmError> {
        let connection = SqliteConnection::connect_with(&config.options)
            .await
            .map_err(|_| OrmError::Driver(DriverError::Connection))?;
        Ok(Self { connection })
    }

    /// Consume the handle and await explicit connection shutdown.
    ///
    /// ```compile_fail
    /// # async fn example(db: crab_driver::SqliteExecutor) {
    /// db.close().await.unwrap();
    /// db.close().await.unwrap(); // the connection was consumed
    /// # }
    /// ```
    pub async fn close(self) -> Result<(), OrmError> {
        self.connection
            .close()
            .await
            .map_err(|_| OrmError::Driver(DriverError::Close))
    }

    async fn prepare(
        &mut self,
        query: CompiledQuery,
        returns_rows: bool,
    ) -> Result<(SqliteStatement, Vec<Value>), OrmError> {
        let lexical_count = input::validate(&query.sql)?;
        // CompiledQuery is a trusted SQL boundary, not a sanitizer. Its caller
        // supplies SQL structure; all portable values are bound separately.
        let statement = self
            .connection
            .prepare(AssertSqlSafe(query.sql).into_sql_str())
            .await
            .map_err(map_error)?;
        let Some(Either::Right(expected)) = statement.parameters() else {
            return Err(OrmError::Driver(DriverError::UnsupportedQuery));
        };
        if lexical_count != expected {
            return Err(OrmError::Driver(DriverError::UnsupportedQuery));
        }
        if expected != query.params.len() {
            return Err(OrmError::Driver(DriverError::ParameterCount {
                expected,
                actual: query.params.len(),
            }));
        }
        if statement.columns().is_empty() == returns_rows {
            return Err(OrmError::Driver(DriverError::UnsupportedQuery));
        }
        Ok((statement, query.params))
    }

    async fn total_changes(&mut self) -> Result<i64, OrmError> {
        sqlx::query_scalar("SELECT total_changes()")
            .fetch_one(&mut self.connection)
            .await
            .map_err(map_error)
    }
}

impl DatabaseExecutor for SqliteExecutor {
    async fn execute(&mut self, query: CompiledQuery) -> Result<ExecutionResult, OrmError> {
        let (statement, params) = self.prepare(query, false).await?;
        let query = bind(&statement, params)?;
        // sqlite3_changes() can retain a preceding DML count for DDL. Normalize
        // non-changing statements to zero without trying to parse SQL kinds.
        let before = self.total_changes().await?;
        let result = query
            .execute(&mut self.connection)
            .await
            .map_err(map_error)?;
        let after = self.total_changes().await?;
        Ok(ExecutionResult {
            rows_affected: if before == after {
                0
            } else {
                result.rows_affected()
            },
        })
    }

    async fn fetch_one(&mut self, query: CompiledQuery) -> Result<Row, OrmError> {
        let (statement, params) = self.prepare(query, true).await?;
        let row = bind(&statement, params)?
            .fetch_one(&mut self.connection)
            .await
            .map_err(map_error)?;
        decode(row)
    }

    async fn fetch_all(&mut self, query: CompiledQuery) -> Result<Vec<Row>, OrmError> {
        let (statement, params) = self.prepare(query, true).await?;
        let rows = bind(&statement, params)?
            .fetch_all(&mut self.connection)
            .await
            .map_err(map_error)?;
        rows.into_iter().map(decode).collect()
    }
}

fn bind(
    statement: &SqliteStatement,
    params: Vec<Value>,
) -> Result<sqlx::query::Query<'_, sqlx::Sqlite, SqliteArguments>, OrmError> {
    let mut query = statement.query();
    for (index, value) in params.into_iter().enumerate() {
        let result = match value {
            Value::Null => query.try_bind(Option::<i64>::None),
            Value::Boolean(value) => query.try_bind(value),
            Value::Integer(value) => query.try_bind(value),
            Value::Real(value) => {
                if !value.is_finite() {
                    return Err(OrmError::Driver(DriverError::InvalidParameter { index }));
                }
                query.try_bind(value)
            }
            Value::Text(value) => query.try_bind(value),
            Value::Blob(value) => query.try_bind(value),
        };
        result.map_err(|_| OrmError::Driver(DriverError::InvalidParameter { index }))?;
    }
    Ok(query)
}

fn decode(row: SqliteRow) -> Result<Row, OrmError> {
    let mut columns = Vec::with_capacity(row.len());
    for (index, column) in row.columns().iter().enumerate() {
        let failure = |_| OrmError::Driver(DriverError::RowConversion { column: index });
        let raw = row.try_get_raw(index).map_err(failure)?;
        // ValueRef's type_info reports the runtime storage class for non-NULL
        // cells, unlike row.columns()' declared metadata. Check NULL first.
        let value = if raw.is_null() {
            Value::Null
        } else {
            match raw.type_info().name() {
                "INTEGER" => Value::Integer(row.try_get(index).map_err(failure)?),
                "REAL" => Value::Real(row.try_get(index).map_err(failure)?),
                "TEXT" => Value::Text(row.try_get(index).map_err(failure)?),
                "BLOB" => Value::Blob(row.try_get(index).map_err(failure)?),
                _ => {
                    return Err(OrmError::Driver(DriverError::RowConversion {
                        column: index,
                    }));
                }
            }
        };
        columns.push(ColumnValue {
            name: column.name().to_owned(),
            value,
        });
    }
    Ok(Row { columns })
}

fn map_error(error: sqlx::Error) -> OrmError {
    use sqlx::error::ErrorKind;
    let kind = match error {
        sqlx::Error::RowNotFound => DriverError::NotFound,
        sqlx::Error::Database(error) => match error.kind() {
            ErrorKind::UniqueViolation => DriverError::Constraint(ConstraintKind::Unique),
            ErrorKind::ForeignKeyViolation => DriverError::Constraint(ConstraintKind::ForeignKey),
            ErrorKind::NotNullViolation => DriverError::Constraint(ConstraintKind::NotNull),
            ErrorKind::CheckViolation => DriverError::Constraint(ConstraintKind::Check),
            ErrorKind::ExclusionViolation => DriverError::Constraint(ConstraintKind::Other),
            _ if error
                .code()
                .and_then(|code| code.parse::<u32>().ok())
                .is_some_and(|code| code & 0xff == 19) =>
            {
                DriverError::Constraint(ConstraintKind::Other)
            }
            _ => DriverError::Execution,
        },
        _ => DriverError::Execution,
    };
    OrmError::Driver(kind)
}
