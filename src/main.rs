use axum::http::StatusCode;
use axum::Server;
use axum::{routing::post, Json, Router};
use axum_sqlite::*;
use regex::Regex;
use rusqlite::Connection;
use serde::Deserialize;
use std::net::SocketAddr;
use std::path::Path;
use validator::Validate;
use validator_derive::Validate;
use validator::ValidationError;

const DATABASE_NAME: &str = "db.sqlite3";

#[derive(Debug, Deserialize, Validate)]
pub struct Robot {
    #[validate(length(min = 1, max = 5))]
    serial: String,
    #[validate(custom = "validate_model_version")]
    model: String,
    #[validate(custom = "validate_model_version")]
    version: String,
    created: String,
}

#[tokio::main]
async fn main() {
    // Создаем маршрутизатор
    let app = Router::new()
        .route("/robots/create", post(create_robot))
        .layer(Database::new(DATABASE_NAME).unwrap());

    // Запускаем сервер на локальном адресе
    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
    // hyper::Server::bind(&addr)
    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

fn setup_database() -> Result<rusqlite::Connection, rusqlite::Error> {
    let conn = Connection::open(DATABASE_NAME)?;
    conn.execute(
            "CREATE TABLE IF NOT EXISTS robots (
                id INTEGER PRIMARY KEY,
                serial TEXT NOT NULL,
                model TEXT NOT NULL,
                version TEXT NOT NULL,
                created TEXT NOT NULL
            )",
        [],
    )?;
    Ok(conn)
}

async fn create_robot(Json(robot): Json<Robot>) -> Result<StatusCode, StatusCode> {
    // Проверяем данные на валидность
    if let Err(_) = robot.validate() {
        return Err(StatusCode::BAD_REQUEST);
    }
    // Открываем соединение с базой данных
    let conn = match Connection::open(Path::new("db.sqlite3")) {
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

fn validate_model_version(value: &str) -> Result<(), ValidationError> {
    // Создаем регулярное выражение для проверки строки
    let re = Regex::new(r"^[A-Za-z][0-9]$").unwrap();
    // Проверяем, что строка соответствует регулярному выражению
    if !re.is_match(value) {
        // Возвращаем ошибку с кодом и сообщением
        println!("Invalid model version");
        return Err(ValidationError::new("invalid_model_version"));
    }
    
    Ok(())
}
