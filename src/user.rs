use anyhow::Result;
use axum::{extract::Json, http::StatusCode};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;
use validator::Validate;
use validator_derive::Validate;

use crate::constants::DATABASE_URL;

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct Customer {
    #[validate(length(min = 1))]
    pub name: String,
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 3))]
    pub login: String,
    #[validate(length(min = 3))]
    pub password: String,
}

async fn open_database() -> Result<PgPool, StatusCode> {
    match PgPool::connect(DATABASE_URL).await {
        Ok(pool) => Ok(pool),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn create_customer(Json(customer): Json<Customer>) -> Result<StatusCode, StatusCode> {
    let pool = open_database().await?;

    if customer.validate().is_err() {
        return Err(StatusCode::BAD_REQUEST);
    }

    let statement = format!(
        "INSERT INTO customers (name, email, login, password) VALUES ($1, $2, $3, $4)"
    );

    match sqlx::query(&statement)
        .bind(&customer.name)
        .bind(&customer.email)
        .bind(&customer.login)
        .bind(&customer.password)
        .execute(&pool)
        .await
    {
        Ok(result) => {
            if result.rows_affected() > 0 {
                println!("User has been added");
                Ok(StatusCode::OK)
            } else {
                println!("Error {result:?}");
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(e) => {
            eprintln!(
                "An error occurred while inserting user into the database: {e}"
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
