use std::path::Path;

use axum::extract::Json;
use serde::{Deserialize, Serialize};
use validator::{Validate, ValidationError};
use validator_derive::Validate;
use rusqlite::Connection;

use crate::db::*;

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct Order {
    // Проверяем, что email имеет правильный формат
    #[validate(email)]
    pub email: String,
    // Проверяем, что модель и версия соответствуют шаблону [A-Za-z][0-9]
    #[validate(custom = "validate_model_version")]
    pub model: String,
    #[validate(custom = "validate_model_version")]
    pub version: String,
}

fn validate_model_version(value: &str) -> Result<(), ValidationError> {
    // Создаем регулярное выражение для проверки строки
    let re = regex::Regex::new(r"^[A-Za-z][0-9]$").unwrap();
    // Проверяем, что строка соответствует регулярному выражению
    if !re.is_match(value) {
        // Возвращаем ошибку с кодом и сообщением
        println!("Invalid model version");
        return Err(ValidationError::new("invalid_model_version"));
    }

    Ok(())
}

pub async fn order_robot(
    Json(order): Json<Order>,
) -> Result<axum::http::StatusCode, axum::http::StatusCode> {
    if order.validate().is_err() {
        return Err(axum::http::StatusCode::BAD_REQUEST);
    }

    let conn = match Connection::open(Path::new(DATABASE_NAME)) {
        Ok(conn) => conn,
        Err(_) => return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    };
    // Формируем запрос на поиск робота по модели и версии
    let statement = format!(
        "SELECT * FROM robots WHERE model = '{}' AND version = '{}'",
        &order.model, &order.version
    );
    // Выполняем запрос и получаем результат
    let result = conn.query_row(&statement, [], |row| row.get::<_, i64>(0));
    // Проверяем, что результат не пустой
    match result {
        Ok(_) => {
            // Робот найден, выводим сообщение в терминал
            println!("product is in stock");
            // Возвращаем статус 200 (OK)
            Ok(axum::http::StatusCode::OK)
        }
        Err(_) => {
            // Робот не найден, выводим сообщение в терминал
            println!("product is out of stock");
            // Возвращаем статус 404 (Not Found)
            Err(axum::http::StatusCode::NOT_FOUND)
        }
    }
}
