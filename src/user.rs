use anyhow::Result;
use axum::{extract::Json, http::StatusCode};
use validator::Validate;

use crate::db::Database;
use crate::structures::Customer;

async fn insert_customer(pool: &sqlx::Pool<sqlx::Postgres>, customer: &Customer) -> Result<u64, sqlx::Error> {
    let statement = format!(
        "INSERT INTO customers (name, email, login, password) VALUES ($1, $2, $3, $4)"
    );

    sqlx::query(&statement)
        .bind(&customer.name)
        .bind(&customer.email)
        .bind(&customer.login)
        .bind(&customer.password)
        .execute(pool)
        .await
        .map(|result| result.rows_affected())
}

pub async fn create_customer(Json(customer): Json<Customer>) -> Result<StatusCode, StatusCode> {
    let db = Database::new().await.unwrap();
    let pool = db.pool;

    if customer.validate().is_err() {
        return Err(StatusCode::BAD_REQUEST);
    }

    match insert_customer(&pool, &customer).await {
        Ok(rows_affected) if rows_affected > 0 => {
            println!("User has been added");
            Ok(StatusCode::OK)
        },
        Ok(_) => {
            println!("User was not added");
            Err(StatusCode::NOT_FOUND)
        },
        Err(e) => {
            eprintln!("An error occurred while inserting user into the database: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        },
    }
}
