use anyhow::Result;
use axum::http::StatusCode;
use chrono::Utc;

use crate::db::Database;

pub async fn add_order(
    customer_name: String,
    robot_model: String,
) -> Result<StatusCode, StatusCode> {
    let db = Database::new().await.unwrap();
    let pool = db.pool;

    let order_date = Utc::now().to_rfc3339();

    let statement = format!(
        r#"INSERT INTO orders (customer_name, robot_model, order_date) VALUES ($1, $2, '{order_date}')"#
    );

    match sqlx::query(&statement)
        .bind(&customer_name)
        .bind(&robot_model)
        .execute(&pool)
        .await
    {
        Ok(result) => {
            if result.rows_affected() > 0 {
                println!("Order has been added");
                Ok(StatusCode::OK)
            } else {
                println!("Error {result:?}");
                Err(StatusCode::NOT_FOUND)
            }
        }
        Err(e) => {
            eprintln!("An error occurred while inserting order into the database: {e}");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}
