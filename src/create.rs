use axum::{http::StatusCode, Json};
use chrono::Utc;
use sqlx::postgres::PgPool;
use validator::Validate;

use crate::constants::DATABASE_URL;
use crate::db::setup_database;
use crate::structures::Robot;

pub async fn generate_serial_number(model: &str) -> Result<String, sqlx::Error> {
    let pool = PgPool::connect(DATABASE_URL).await?;

    // let sql = "SELECT COUNT(*) as count FROM robots WHERE model = ?";
    let sql = "SELECT COUNT(*) as count FROM robots WHERE model = $1";
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

async fn open_database() -> Result<PgPool, StatusCode> {
    match PgPool::connect(DATABASE_URL).await {
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
    // FIX IT
    // It does not work with $4, but works with hardcoded date like this
    // The curl all the time is the same:
    // curl -X POST -H "Content-Type: application/json" -d '{"serial":"0","model":"A1","version":"A1","created":"2023-09-06 11:09:22"}' http://127.0.0.1:8000/robots/create
    // It worked with SQlite, nut with postgres:
    // An error occurred while inserting data into the database: error returned from database: column "created" is of type timestamp without time zone but expression is of type text
    // let statement = format!("INSERT INTO robots (serial, model, version, created) VALUES ($1, $2, $3, '2023-09-06 11:09:22')");
    let current_date = Utc::now().to_rfc3339();
    let statement = format!("INSERT INTO robots (serial, model, version, created) VALUES ($1, $2, $3, '{}')", current_date);
    // Выполняем запрос и возвращаем статус
    match sqlx::query(&statement)
        .bind(&serial_number)
        .bind(&robot.model)
        .bind(&robot.version)
        .bind(&robot.created)
        .execute(&pool)
        .await
    {
        Ok(_) => {
            pool.close().await;
            Ok(StatusCode::CREATED)
        }
        // Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
        Err(e) => {
            // Выводим информацию об ошибке и вызываем панику
            eprintln!(
                "An error occurred while inserting data into the database: {}",
                e
            );
            panic!("Database error");
        }
    }
}

pub async fn remove_robot(Json(robot): Json<Robot>) -> Result<StatusCode, StatusCode> {
    validate_robot(&robot)?;

    let pool = open_database().await?;

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
