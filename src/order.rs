use std::time::Duration;
use std::{path::Path, sync::Arc};

use axum::extract::Json;
use lettre::transport::smtp::response::Response;
use lettre::{Message, SmtpTransport, Transport};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use tokio::time::sleep;
use validator::{Validate, ValidationError};
use validator_derive::Validate;

use crate::constants::{CHECK_INTERVAL, DATABASE_NAME, SMTP_SENDER, SMTP_SERVER};
use crate::structures::Customer;

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct Order {
    pub email: Customer,
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

// Формируем запрос на поиск робота по модели и версии
// Выполняем запрос и получаем результат
fn find_robot_in_db(conn: &Connection, model: &str, version: &str) -> rusqlite::Result<i64> {
    let statement = format!(
        "SELECT * FROM robots WHERE model = '{}' AND version = '{}'",
        model, version
    );
    conn.query_row(&statement, [], |row| row.get::<_, i64>(0))
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

        let result = find_robot_in_db(&conn, &order.model, &order.version);
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
        // Запускаем бесконечный цикл
        loop {
            // Получаем доступ к соединению с базой данных
            let conn = self.conn.lock().await;
            // Создаем пустой вектор для хранения заказов, которые еще не выполнены
            let mut pending_orders = Vec::new();
            println!("LOOP\t{pending_orders:?}");
            // Итерируем по вектору заказов с помощью метода drain, который перемещает элементы из вектора
            for order in self.orders.drain(..) {
                let result = find_robot_in_db(&conn, &order.model, &order.version);

                match result {
                    Ok(_) => {
                        // Робот найден, выводим сообщение в терминал
                        println!("product is available");
                        // Сообщение для заказчика
                        let message = format!(
                            "Добрый день!\n\
                            Недавно вы интересовались нашим роботом модели {}, версии {}.\n\
                            Этот робот теперь в наличии. Если вам подходит этот вариант - пожалуйста, свяжитесь с нами",
                            &order.model, &order.version
                        );
                        send_email("customer@test.org", &message).unwrap();
                        println!("{}", message);
                        // Заказ обратно в вектор, так как он выполнен
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
            sleep(Duration::from_secs(CHECK_INTERVAL)).await;
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
    let queue = Arc::new(Mutex::new(OrderQueue::new()));
    // Получаем блокировку на очередь и добавляем заказ
    queue.lock().await.enqueue(order).await;
    // Запускаем задачу для обработки очереди
    let queue_clone = Arc::clone(&queue);
    tokio::spawn(async move {
        // Получаем блокировку на очередь и вызываем метод process
        queue_clone.lock().await.process().await;
    });

    // Проверяем, есть ли заказы в очереди
    if queue.lock().await.orders.is_empty() {
        // Если очередь пуста, возвращаем статус код 200 (OK)
        Ok(axum::http::StatusCode::OK)
    } else {
        // Если в очереди есть заказы, возвращаем статус код 404 (Not Found)
        Err(axum::http::StatusCode::NOT_FOUND)
    }
}

fn send_email(to: &str, body: &str) -> Result<Response, lettre::transport::smtp::Error> {
    let email = Message::builder()
        .from(SMTP_SENDER.parse().unwrap())
        .to(to.parse().unwrap())
        .subject("Your order is available")
        .body(body.to_string())
        .unwrap();

    let mailer = SmtpTransport::relay(SMTP_SERVER)
        .unwrap()
        .credentials(lettre::transport::smtp::authentication::Credentials::new(
            "user".to_string(),
            "password".to_string(),
        ))
        .build();

    mailer.send(&email)
}
