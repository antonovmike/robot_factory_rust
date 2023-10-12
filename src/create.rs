use axum::{http::StatusCode, Json};
use sqlx::sqlite::SqlitePool;
use validator::Validate;

use crate::constants::DATABASE_NAME;
use crate::db::setup_database;
use crate::structures::Robot;

pub async fn generate_serial_number(model: &str) -> Result<String, sqlx::Error> {
    let pool = SqlitePool::connect(DATABASE_NAME).await?;

    let sql = "SELECT COUNT(*) as count FROM robots WHERE model = ?";
    let max_serial: Option<i64> = sqlx::query_scalar(sql).bind(model).fetch_one(&pool).await?;
    let new_serial = format!("{}{:03}", model, max_serial.unwrap_or(0) + 1);

    Ok(new_serial)
}

fn validate_robot(robot: &Robot) -> Result<(), StatusCode> {
    if robot.validate().is_err() {
        Err(StatusCode::BAD_REQUEST)
    } else {
        Ok(())
    }
}

async fn open_database() -> Result<SqlitePool, StatusCode> {
    match SqlitePool::connect(DATABASE_NAME).await {
        Ok(pool) => Ok(pool),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn create_robot(Json(robot): Json<Robot>) -> Result<StatusCode, StatusCode> {
    validate_robot(&robot)?;

    let pool = open_database().await?;

    let serial_number;
    if robot.serial == "0" {
        serial_number = generate_serial_number(&robot.model).await.unwrap();
    } else {
        serial_number = robot.serial
    }
    println!("Serial number: {serial_number}");

    match setup_database().await {
        Ok(_) => (),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    let statement =
        format!("INSERT INTO robots (serial, model, version, created) VALUES ($1, $2, $3, $4)");
    // Выполняем запрос и возвращаем статус
    match sqlx::query(&statement)
        .bind(&serial_number)
        .bind(&robot.model)
        .bind(&robot.version)
        .bind(&robot.created)
        .execute(&pool)
        .await
    {
        Ok(_) => Ok(StatusCode::CREATED),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn remove_robot(Json(robot): Json<Robot>) -> Result<StatusCode, StatusCode> {
    validate_robot(&robot)?;

    let pool = open_database().await?;

    // let statement = format!("DELETE FROM robots WHERE serial = $1 AND model = $2 AND version = $3");
    let statement = format!("DELETE FROM robots WHERE serial = $1");

    match sqlx::query(&statement)
        .bind(&robot.serial)
        .bind(&robot.model)
        .bind(&robot.version)
        .execute(&pool)
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
