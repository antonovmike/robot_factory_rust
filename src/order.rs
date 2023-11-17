use anyhow::Result;
use axum::http::StatusCode;
use chrono::Utc;

use crate::db_pool::get_pool;

pub struct Order {
    pub customer_name: String,
    pub robot_model: String,
}

impl Order {
    pub async fn add_order(
        &self,
        customer_name: String,
        robot_model: String,
    ) -> Result<StatusCode, StatusCode> {
        let pool = get_pool().await.unwrap();

        let order_date = Utc::now().to_rfc3339();

        let statement = format!(
            r#"INSERT INTO orders (customer_name, robot_model, order_date) VALUES (\$1, \$2, '{}')"#,
            order_date
        );

        match sqlx::query(&statement)
            .bind(&customer_name)
            .bind(&robot_model)
            .execute(&*pool)
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
}
