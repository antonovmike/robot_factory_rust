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
    // sqlx::query(
    //     "CREATE TABLE IF NOT EXISTS robots (
    //     id SERIAL PRIMARY KEY,
    //     serial TEXT NOT NULL,
    //     model TEXT NOT NULL,
    //     version TEXT NOT NULL,
    //     created TIMESTAMP NOT NULL)",
    // )
    // .execute(&pool)
    // .await
    // .unwrap();

    Ok(pool)
}

pub async fn get_robots_by_date(date: &str) -> Result<i64, sqlx::Error> {
    let pool = PgPool::connect(DATABASE_URL).await?;
    // let count: (i64,) = sqlx::query_as(r"SELECT COUNT(*) FROM robots WHERE created <= $1")
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
