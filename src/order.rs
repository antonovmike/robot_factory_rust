use std::{path::Path, sync::Arc};

use axum::extract::Json;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use validator::{Validate, ValidationError};
use validator_derive::Validate;

use crate::{db::*, OrderQueue};

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
    // Создаем экземпляр очереди
    let mut queue = OrderQueue::new();
    queue.enqueue(order).await;
    // Оборачиваем очередь в Arc и Mutex для безопасного доступа из разных потоков
    let queue = Arc::new(Mutex::new(queue));
    // Клонируем ссылку на очередь для передачи в обработчик /robots/order
    // let queue_clone = queue.clone();
    // Запускаем задачу для обработки очереди
    tokio::spawn(async move {
        // Получаем доступ к очереди
        let mut queue = queue.lock().await;
        // Вызываем метод process для обработки очереди
        queue.process().await;
    });

    // if order.validate().is_err() {
    //     return Err(axum::http::StatusCode::BAD_REQUEST);
    // }

    let conn = match Connection::open(Path::new(DATABASE_NAME)) {
        Ok(conn) => conn,
        Err(_) => return Err(axum::http::StatusCode::INTERNAL_SERVER_ERROR),
    };
    // Формируем запрос на поиск робота по модели и версии
    let statement = format!(
        "SELECT * FROM robots WHERE model = 'X0' AND version = 'X0'",
        // &order.model, &order.version
    );
    // Выполняем запрос и получаем результат
    let result = conn.query_row(&statement, [], |row| row.get::<_, i64>(0));
    // Проверяем, что результат не пустой
    match result {
        Ok(_) => {
            // Робот найден, выводим сообщение в терминал
            println!("Product is in stock");
            // Возвращаем статус 200 (OK)
            Ok(axum::http::StatusCode::OK)
        }
        Err(_) => {
            // Робот не найден, выводим сообщение в терминал
            println!("Product is out of stock");
            // Возвращаем статус 404 (Not Found)
            Err(axum::http::StatusCode::NOT_FOUND)
        }
    }
}
