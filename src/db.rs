use regex::Regex;
use sqlx::{sqlite::SqlitePool, Error, Executor};
use validator::ValidationError;

use crate::constants::DATABASE_NAME;

pub async fn setup_database() -> Result<SqlitePool, Error> {
    // Создаем пул соединений с базой данных
    let pool = SqlitePool::connect(&format!("sqlite://{}", DATABASE_NAME)).await?;
    // Запрос на создание таблицы, если она не существует
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

pub async fn get_robots_by_date(date: &str) -> Result<i64, sqlx::Error> {
    // Открываем соединение с базой данных
    let pool = SqlitePool::connect(&format!("sqlite://{}", DATABASE_NAME)).await?;
    // Формируем запрос на выборку суммы всех роботов до даты и времени создания
    let count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM robots WHERE created <= datetime(?)",
    )
    .bind(date)
    .fetch_one(&pool)
    .await?;

    Ok(count.0)
}

pub fn validate_model_version(value: &str) -> Result<(), ValidationError> {
    // Регулярное выражение для проверки строки
    let re = Regex::new(r"^[A-Za-z][0-9]$").unwrap();

    if !re.is_match(value) {
        println!("Invalid model version");
        return Err(ValidationError::new("invalid_model_version"));
    }

    Ok(())
}
