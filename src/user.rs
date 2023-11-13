use anyhow::Result;
use axum::Extension;
use axum::{extract::Json, http::StatusCode};
use sqlx::PgPool;
use validator::Validate;

use crate::structures::Customer;

impl Customer {
    async fn insert(&self, pool: &sqlx::Pool<sqlx::Postgres>) -> Result<u64, sqlx::Error> {
        let statement =
            ("INSERT INTO customers (name, email, login, password) VALUES ($1, $2, $3, $4)")
                .to_string();

        sqlx::query(&statement)
            .bind(&self.name)
            .bind(&self.email)
            .bind(&self.login)
            .bind(&self.password)
            .execute(pool)
            .await
            .map(|result| result.rows_affected())
    }
}

// in Axum 0.6.0 and later, the extractor that consumes the request body
// must be last in the list of route handler arguments.
// This means that Json<Customer> must be the last argument in the route handler
pub async fn create_customer(
    (Extension(pool), Json(customer)): (Extension<PgPool>, Json<Customer>),
) -> Result<StatusCode, StatusCode> {
    if customer.validate().is_err() {
        return Err(StatusCode::BAD_REQUEST);
    }

    match customer.insert(&pool).await {
        Ok(rows_affected) if rows_affected > 0 => {
            println!("User has been added");
            Ok(StatusCode::OK)
        }
        Ok(_) => {
            println!("User was not added");
            Err(StatusCode::NOT_FOUND)
        }
        Err(e) => {
            eprintln!(
                "An error occurred while inserting user into the database: {}",
                e
            );
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
