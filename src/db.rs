use std::path::Path;

use regex::Regex;
use rusqlite::{Connection, Result};
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};
use validator_derive::Validate;

pub const DATABASE_NAME: &str = "db.sqlite3";

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct Robot {
    #[validate(length(min = 1, max = 5))]
    pub serial: String,
    #[validate(custom = "validate_model_version")]
    pub model: String,
    #[validate(custom = "validate_model_version")]
    pub version: String,
    pub created: String,
}

pub fn setup_database() -> Result<rusqlite::Connection, rusqlite::Error> {
    let conn = Connection::open(DATABASE_NAME)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS robots (
id INTEGER PRIMARY KEY,
serial TEXT NOT NULL,
model TEXT NOT NULL,
version TEXT NOT NULL,
created TEXT NOT NULL
)",
        [],
    )?;
    Ok(conn)
}

pub fn get_robots_by_date(date: &str) -> Result<Vec<Robot>, rusqlite::Error> {
    // Открываем соединение с базой данных
    let conn = Connection::open(Path::new("db.sqlite3"))?;
    // Формируем запрос на выборку роботов по дате создания
    let statement = format!(
        "SELECT serial, model, version, created FROM robots 
WHERE created >= date('{}')",
        date
    );
    // Выполняем запрос и получаем итератор по строкам
    let mut stmt = conn.prepare(&statement)?;
    let rows = stmt.query_map([], |row| {
        Ok(Robot {
            serial: row.get(0)?,
            model: row.get(1)?,
            version: row.get(2)?,
            created: row.get(3)?,
        })
    })?;
    // Собираем строки в вектор роботов
    let mut robots = Vec::new();
    for row in rows {
        robots.push(row?);
    }
    Ok(robots)
}

fn validate_model_version(value: &str) -> Result<(), ValidationError> {
    // Создаем регулярное выражение для проверки строки
    let re = Regex::new(r"^[A-Za-z][0-9]$").unwrap();
    // Проверяем, что строка соответствует регулярному выражению
    if !re.is_match(value) {
        // Возвращаем ошибку с кодом и сообщением
        println!("Invalid model version");
        return Err(ValidationError::new("invalid_model_version"));
    }

    Ok(())
}
