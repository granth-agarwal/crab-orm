//! Inspect a manually constructed contract through the facade; no database I/O.

use crab_orm::{CompiledQuery, Value};

fn main() {
    let query = CompiledQuery {
        sql: "SELECT id, title, completed FROM todos WHERE completed = ? AND id > ?".into(),
        params: vec![Value::Boolean(false), Value::Integer(0)],
    };

    println!("CrabORM Phase 0: owned query contract through the crab-orm facade.");
    // This hard-coded parameterized SQL is safe to display explicitly.
    println!("SQL: {}", query.sql);
    println!(
        "Ordered bind count: {} (values redacted)",
        query.params.len()
    );
    println!("SQLite execution and CRUD will arrive in Phase 1. No connection was opened.");
}
