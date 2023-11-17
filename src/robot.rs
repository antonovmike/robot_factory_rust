use axum::http::StatusCode;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use validator::Validate;
use validator_derive::Validate;

use crate::db::{Database, validate_model_version};
use crate::db_pool::get_pool;

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct Robot {
    #[validate(length(min = 1, max = 5))]
    pub serial: String,
    #[validate(custom = "validate_model_version")]
    pub model: String,
    #[validate(custom = "validate_model_version")]
    pub version: String,
}

impl Robot {
    pub async fn generate_serial_number(model: &str) -> Result<String, sqlx::Error> {
        let pool = get_pool().await?;

        println!("generate serial {model:?}");
        let sql = "SELECT COUNT(*) as count FROM robots WHERE model = $1";
        let max_serial: Option<i64> = sqlx::query_scalar(sql).bind(model).fetch_one(&*pool).await?;
        let new_serial = format!("{}{:03}", model, max_serial.unwrap_or(0) + 1);

        Ok(new_serial)
    }

    pub fn validate_robot(&self) -> Result<(), StatusCode> {
        if self.validate().is_err() {
            Err(StatusCode::BAD_REQUEST)
        } else {
            Ok(())
        }
    }

    pub async fn create_robot(&self) -> Result<StatusCode, StatusCode> {
        self.validate_robot()?;
        println!("create_robot");
        let db = Database::new().await?;
        let pool = db.pool;

        let serial_number = if self.serial == "0" {
            Self::generate_serial_number(&self.model).await.unwrap()
        } else {
            self.serial.clone()
        };
        println!("Serial number: {serial_number}");

        let current_date = Utc::now().to_rfc3339();
        let statement = format!(
            r#"INSERT INTO robots (serial, model, version, created) VALUES ($1, $2, $3, '{}')"#,
            current_date
        );
        // Execute the request and return the status
        match sqlx::query(&statement)
            .bind(&serial_number)
            .bind(&self.model)
            .bind(&self.version)
            .execute(&pool)
            .await
        {
            Ok(_) => {
                pool.close().await;
                Ok(StatusCode::CREATED)
            }
            Err(e) => {
                eprintln!("An error occurred while inserting data into the database: {e}");
                panic!("Database error: {e}");
            }
        }
    }

    pub async fn remove_robot(&self) -> Result<StatusCode, StatusCode> {
        self.validate_robot()?;

        let pool = get_pool().await.unwrap();

        let statement = ("DELETE FROM robots WHERE serial = $1").to_string();

        match sqlx::query(&statement)
            .bind(&self.serial)
            .bind(&self.model)
            .bind(&self.version)
            .execute(&*pool)
            .await
        {
            Ok(result) => {
                if result.rows_affected() > 0 {
                    println!("Robot has been removed");
                    Ok(StatusCode::OK)
                } else {
                    println!("Robot not found");
                    Err(StatusCode::NOT_FOUND)
                }
            }
            Err(_) => {
                println!("An error occurred while attempting to remove the robot");
                Err(StatusCode::INTERNAL_SERVER_ERROR)
            }
        }
    }
}
