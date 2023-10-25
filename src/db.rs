use regex::Regex;
use sqlx::{postgres::PgPool, Error, Executor};
use validator::ValidationError;

use crate::constants::DATABASE_URL;

pub async fn setup_database() -> Result<PgPool, Error> {
    let pool = PgPool::connect(DATABASE_URL).await?;

    pool.execute(
        "CREATE TABLE IF NOT EXISTS robots (
        id SERIAL PRIMARY KEY,
        serial TEXT NOT NULL,
        model TEXT NOT NULL,
        version TEXT NOT NULL,
        created TIMESTAMP NOT NULL
        )",
    )
    .await?;

    pool.execute(
        "CREATE TABLE IF NOT EXISTS customers (
        id SERIAL PRIMARY KEY,
        name TEXT NOT NULL,
        email TEXT NOT NULL UNIQUE,
        login TEXT NOT NULL UNIQUE,
        password TEXT NOT NULL
        )",
    )
    .await?;

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS orders (
        id SERIAL PRIMARY KEY,
        customer_id INTEGER NOT NULL,
        robot_id INTEGER NOT NULL,
        order_date TIMESTAMP NOT NULL,
        CONSTRAINT fk_customer FOREIGN KEY (customer_id) REFERENCES customers (id),
        CONSTRAINT fk_robot FOREIGN KEY (robot_id) REFERENCES robots (id)
        )",
    )
    .execute(&pool)
    .await?;

    Ok(pool)
}

pub async fn get_robots_by_date(date: &str) -> Result<i64, sqlx::Error> {
    let pool = PgPool::connect(DATABASE_URL).await?;

    let count: (i64,) = sqlx::query_as(
        r"SELECT COUNT(*) FROM robots WHERE created <= TO_TIMESTAMP($1, 'YYYY-MM-DD HH24:MI:SS')",
    )
    .bind(date)
    .fetch_one(&pool)
    .await?;

    Ok(count.0)
}

pub fn validate_model_version(value: &str) -> Result<(), ValidationError> {
    let re = Regex::new(r"^[A-Za-z][0-9]$").unwrap();

    if !re.is_match(value) {
        println!("Invalid model version");
        return Err(ValidationError::new("invalid_model_version"));
    }

    Ok(())
}

// Проверка логина и пароля в базе данных
// Если найден - возвращаем email пользователя
pub async fn check_credentials(login: &str, password: &str) -> Result<Option<String>, sqlx::Error> {
    let pool = PgPool::connect(DATABASE_URL).await?;

    let sql = "SELECT email FROM customers WHERE login = $1 AND password = $2";

    sqlx::query_scalar(sql)
        .bind(login)
        .bind(password)
        .fetch_optional(&pool)
        .await
}