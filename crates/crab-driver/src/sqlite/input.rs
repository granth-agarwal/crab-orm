//! A lexical boundary guard, not a SQL grammar or statement classifier.
//!
//! Accept one statement and anonymous `?` binds only. SQLite/SQLx validates SQL
//! syntax and supplies authoritative prepared parameter metadata afterward.

use crab_core::{DriverError, OrmError};

pub(super) fn validate(sql: &str) -> Result<usize, OrmError> {
    let bytes = sql.as_bytes();
    if bytes.contains(&0) {
        return Err(OrmError::Driver(DriverError::InvalidQuery));
    }
    let mut index = 0;
    let mut parameters = 0;
    let mut content = false;
    let mut terminated = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte.is_ascii_whitespace() {
            index += 1;
            continue;
        }
        if bytes[index..].starts_with(b"--") {
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if bytes[index..].starts_with(b"/*") {
            index += 2;
            while index + 1 < bytes.len() && !bytes[index..].starts_with(b"*/") {
                index += 1;
            }
            if index + 1 >= bytes.len() {
                return Err(OrmError::Driver(DriverError::InvalidQuery));
            }
            index += 2;
            continue;
        }
        if terminated {
            return Err(OrmError::Driver(DriverError::UnsupportedQuery));
        }
        match byte {
            b';' => {
                if !content {
                    return Err(OrmError::Driver(DriverError::InvalidQuery));
                }
                terminated = true;
                index += 1;
            }
            b'\'' | b'"' | b'`' | b'[' => {
                content = true;
                let close = if byte == b'[' { b']' } else { byte };
                index += 1;
                loop {
                    if index >= bytes.len() {
                        return Err(OrmError::Driver(DriverError::InvalidQuery));
                    }
                    if bytes[index] == close {
                        index += 1;
                        if close != b']' && bytes.get(index) == Some(&close) {
                            index += 1;
                        } else {
                            break;
                        }
                    } else {
                        index += 1;
                    }
                }
            }
            b'?' => {
                if bytes.get(index + 1).is_some_and(u8::is_ascii_digit) {
                    return Err(OrmError::Driver(DriverError::UnsupportedQuery));
                }
                content = true;
                parameters += 1;
                index += 1;
            }
            b':' | b'@' | b'$' => {
                return Err(OrmError::Driver(DriverError::UnsupportedQuery));
            }
            _ => {
                content = true;
                index += 1;
            }
        }
    }
    if !content {
        return Err(OrmError::Driver(DriverError::InvalidQuery));
    }
    Ok(parameters)
}
