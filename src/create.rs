use std::path::Path;

use axum::{http::StatusCode, Json};
use rusqlite::{Connection, Result};
use validator::Validate;

use crate::constants::DATABASE_NAME;
use crate::structures::Robot;
use crate::db::setup_database;

pub async fn create_robot(Json(robot): Json<Robot>) -> Result<StatusCode, StatusCode> {
    // Проверяем данные на валидность
    if robot.validate().is_err() {
        return Err(StatusCode::BAD_REQUEST);
    }
    // Открываем соединение с базой данных
    let conn = match Connection::open(Path::new(DATABASE_NAME)) {
        Ok(conn) => conn,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };
    // Создаем таблицу robots, если ее не существует
    match setup_database() {
        Ok(_) => (),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };
    // Формируем запрос на вставку данных о роботе
    let statement = format!(
        "INSERT INTO robots (serial, model, version, created) VALUES ('{}', '{}', '{}', '{}')",
        &robot.serial, &robot.model, &robot.version, &robot.created
    );
    // Выполняем запрос и возвращаем статус
    match conn.execute(&statement, []) {
        Ok(_) => Ok(StatusCode::CREATED),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn remove_robot(Json(robot): Json<Robot>) -> Result<StatusCode, StatusCode> {
    // Проверяем данные на валидность
    if robot.validate().is_err() {
        return Err(StatusCode::BAD_REQUEST);
    }
    let conn = match Connection::open(Path::new(DATABASE_NAME)) {
        Ok(conn) => conn,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };
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
        },
        Err(_) => {
            println!("An error occurred while attempting to remove the robot");
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        },
    }
}

