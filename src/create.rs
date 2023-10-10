use std::path::Path;

use axum::{http::StatusCode, Json};
use rusqlite::{params, Connection, Result};
use validator::Validate;

use crate::constants::DATABASE_NAME;
use crate::db::setup_database;
use crate::structures::Robot;

pub fn generate_serial_number(model: &str) -> Result<String, rusqlite::Error> {
    let conn = Connection::open(DATABASE_NAME)?;

    let sql = "SELECT COUNT(*) as count FROM robots WHERE model = ?";
    let mut stmt = conn.prepare(&sql)?;
    
    let max_serial: Option<i32> = stmt.query_row(params![model], |row| row.get(0))?;
    let new_serial = format!("{}{:04}", model, max_serial.unwrap_or(0) + 1);
    
    Ok(new_serial)    
}

// Проверяем данные на валидность
fn validate_robot(robot: &Robot) -> Result<(), StatusCode> {
    if robot.validate().is_err() {
        Err(StatusCode::BAD_REQUEST)
    } else {
        Ok(())
    }
}

// Открываем соединение с базой данных
fn open_database() -> Result<Connection, StatusCode> {
    match Connection::open(Path::new(DATABASE_NAME)) {
        Ok(conn) => Ok(conn),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn create_robot(Json(robot): Json<Robot>) -> Result<StatusCode, StatusCode> {
    validate_robot(&robot)?;

    let conn = open_database()?;

    let mut serial_number = String::new();
    if robot.serial == "0" {
        serial_number = generate_serial_number(&robot.model).unwrap();
    } else {
        serial_number = robot.serial
    }
    println!("serial_number: {serial_number}");

    // Создаем таблицу robots, если ее не существует
    match setup_database() {
        Ok(_) => (),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };
    // Формируем запрос на вставку данных о роботе
    let statement = format!(
        "INSERT INTO robots (serial, model, version, created) VALUES ('{}', '{}', '{}', '{}')",
        &serial_number, &robot.model, &robot.version, &robot.created
    );
    // Выполняем запрос и возвращаем статус
    match conn.execute(&statement, []) {
        Ok(_) => Ok(StatusCode::CREATED),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn remove_robot(Json(robot): Json<Robot>) -> Result<StatusCode, StatusCode> {
    validate_robot(&robot)?;

    let conn = open_database()?;

    let statement = format!(
        "DELETE FROM robots WHERE serial = '{}' AND model = '{}' AND version = '{}'",
        &robot.serial, &robot.model, &robot.version
    );
    
    match conn.execute(&statement, []) {
        Ok(rows_affected) => {
            if rows_affected > 0 {
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
