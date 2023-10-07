use std::{path::Path, sync::Arc};
use std::time::Duration;

use axum::extract::Json;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tokio::time::sleep;
use tokio::sync::Mutex;
use validator::{Validate, ValidationError};
use validator_derive::Validate;

use crate::constants::DATABASE_NAME;

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

// Структура для представления очереди заказов
pub struct OrderQueue {
    // Вектор заказов
    pub orders: Vec<Order>,
    // Ссылка на соединение с базой данных
    conn: Arc<Mutex<Connection>>,
}

impl OrderQueue {
    // Метод для создания нового экземпляра очереди
    pub fn new() -> Self {
        // Открываем соединение с базой данных
        let conn = match Connection::open(Path::new(DATABASE_NAME)) {
            Ok(conn) => conn,
            Err(err) => panic!("Failed to open database connection: {}", err),
        };
        // Оборачиваем соединение в Arc и Mutex для безопасного доступа из разных потоков
        let conn = Arc::new(Mutex::new(conn));
        // Создаем пустой вектор заказов
        let orders = Vec::new();
        // Возвращаем новый экземпляр очереди
        Self { orders, conn }
    }

    // Метод для добавления заказа в очередь
    pub async fn enqueue(&mut self, order: Order) {
        println!("enqueue: {order:?}");
        // Получаем доступ к соединению с базой данных
        let conn = self.conn.lock().await;
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
            }
            Err(_) => {
                // Робот не найден, выводим сообщение в терминал
                println!("product is out of stock");
                // Добавляем заказ в вектор
                self.orders.push(order);
            }
        }
    }

    // Метод для обработки очереди
    pub async fn process(&mut self) {
        // Задаем интервал проверки в секундах
        let interval = 4;
        // Запускаем бесконечный цикл
        loop {
            // Получаем доступ к соединению с базой данных
            let conn = self.conn.lock().await;
            // Создаем пустой вектор для хранения заказов, которые еще не выполнены
            let mut pending_orders = Vec::new();
            println!("LOOP\t{pending_orders:?}");
            // Итерируем по вектору заказов с помощью метода drain, который перемещает элементы из вектора
            for order in self.orders.drain(..) {
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
                        println!("product is available");
                        // Не добавляем заказ обратно в вектор, так как он выполнен
                    }
                    Err(_) => {
                        // Робот не найден, добавляем заказ в вектор для дальнейшей обработки
                        pending_orders.push(order);
                    }
                }
            }
            // Заменяем вектор заказов на вектор невыполненных заказов
            self.orders = pending_orders;
            // Освобождаем доступ к соединению с базой данных
            drop(conn);
            // Ждем заданный интервал времени
            sleep(Duration::from_secs(interval)).await;
        }
    }
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
    // Запускаем задачу для обработки очереди
    tokio::spawn(async move {
        // Получаем доступ к очереди
        let mut queue = queue.lock().await;
        // Вызываем метод process для обработки очереди
        queue.process().await;
    });

    // EDIT THIS PART

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
