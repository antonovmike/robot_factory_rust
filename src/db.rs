// use std::path::Path;

use regex::Regex;
use sqlx::{sqlite::SqlitePool, Error, Executor};
use validator::ValidationError;

use crate::constants::DATABASE_NAME;

pub async fn setup_database() -> Result<SqlitePool, Error> {
    // Создаем пул соединений с базой данных
    let pool = SqlitePool::connect(&format!("sqlite://{}", DATABASE_NAME)).await?;
    // Выполняем запрос на создание таблицы, если она не существует
    pool.execute(
        "CREATE TABLE IF NOT EXISTS robots (
        id INTEGER PRIMARY KEY,
        serial TEXT NOT NULL,
        model TEXT NOT NULL,
        version TEXT NOT NULL,
        created TEXT NOT NULL
    )",
    )
    .await?;
    Ok(pool)
}

// pub fn get_robots_by_date(date: &str) -> Result<i64, rusqlite::Error> {
//     // Открываем соединение с базой данных
//     let conn = Connection::open(Path::new("db.sqlite3"))?;
//     // Формируем запрос на выборку суммы всех роботов до даты и времени создания
//     let statement = format!(
//         "SELECT COUNT(*) FROM robots 
//     WHERE created <= datetime('{}')",
//         date
//     );
//     // Выполняем запрос и получаем одно число из первой строки и первого столбца
//     let count: i64 = conn.query_row(&statement, [], |row| row.get(0))?;
//     // Возвращаем количество роботов
//     Ok(count)
// }

pub fn validate_model_version(value: &str) -> Result<(), ValidationError> {
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
