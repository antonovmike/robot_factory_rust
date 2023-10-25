use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use axum::extract::Json;
use lettre::transport::smtp::response::Response;
use lettre::{Message, SmtpTransport, Transport};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;
use tokio::sync::Mutex;
use tokio::time::sleep;
use validator::{Validate, ValidationError};
use validator_derive::Validate;

use crate::constants::{CHECK_INTERVAL, DATABASE_URL, SMTP_SENDER, SMTP_SERVER};
use crate::db::check_credentials;

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct Order {
    // pub email: String,
    pub login: String,
    pub password: String,
    // Проверяем, что модель и версия соответствуют шаблону [A-Za-z][0-9]
    #[validate(custom = "validate_model_version")]
    pub model: String,
    #[validate(custom = "validate_model_version")]
    pub version: String,
}

pub struct OrderQueue {
    pub orders: std::collections::VecDeque<Order>,
    // Ссылка на соединение с базой данных
    pool: Arc<Mutex<PgPool>>,
}

// Формируем запрос на поиск робота по модели и версии
// Выполняем запрос и получаем результат
async fn find_robot_in_db(pool: &PgPool, model: &str, version: &str) -> sqlx::Result<i32> {
    let sql = "SELECT * FROM robots WHERE model = $1 AND version = $2";

    sqlx::query_scalar(sql)
        .bind(model)
        .bind(version)
        .fetch_one(pool)
        .await
}

impl OrderQueue {
    pub async fn new() -> Self {
        let pool = match PgPool::connect(DATABASE_URL).await {
            Ok(pool) => pool,
            Err(err) => panic!("Failed to open database connection: {}", err),
        };
        // Оборачиваем соединение в Arc и Mutex для безопасного доступа из разных потоков
        let pool = Arc::new(Mutex::new(pool));

        let orders = VecDeque::new();

        Self { orders, pool }
    }

    // Метод для добавления заказа в очередь
    pub async fn enqueue(&mut self, order: Order) {
        println!("enqueue: {order:?}");
        // Получаем доступ к соединению с базой данных
        let pool = self.pool.lock().await;

        let result = find_robot_in_db(&pool, &order.model, &order.version).await;
        // Проверяем, что результат не пустой
        match result {
            Ok(_) => {
                println!("product is in stock");
            }
            Err(_) => {
                println!("product is out of stock");
                // Добавляем заказ в вектор
                // self.orders.push(order);
                self.orders.push_back(order);
            }
        }
    }

    // Метод для обработки очереди
    pub async fn process(&mut self) {
        // Запускаем бесконечный цикл
        loop {
            let pool = self.pool.lock().await;
            // Создаем пустой вектор для хранения заказов, которые еще не выполнены
            let mut pending_orders = VecDeque::new();

            // Итерируем по вектору заказов с помощью метода drain, который перемещает элементы из вектора
            for _ in 0..self.orders.len() {
                let order = self.orders.pop_front().unwrap();
                let result = find_robot_in_db(&pool, &order.model, &order.version).await;
                match result {
                    Ok(_) => {
                        println!("product is available");

                        let message = format!(
                            "Добрый день!\n\
                            Недавно вы интересовались нашим роботом модели {}, версии {}.\n\
                            Этот робот теперь в наличии. Если вам подходит этот вариант - пожалуйста, свяжитесь с нами",
                            &order.model, &order.version
                        );

                        let (login, password) = (order.login, order.password);
                        let email_addr = check_credentials(&login, &password).await.unwrap().unwrap();

                        send_email(&email_addr, &message).expect("Failed to send email");

                        println!("{}", message);
                    }
                    Err(_) => {
                        pending_orders.push_back(order);
                    }
                }
            }
            // Заменяем вектор заказов на вектор невыполненных заказов
            self.orders = pending_orders;
            // Освобождаем доступ к соединению с базой данных
            drop(pool);

            sleep(Duration::from_secs(CHECK_INTERVAL)).await;
        }
    }
}

fn validate_model_version(value: &str) -> Result<(), ValidationError> {
    // Регулярное выражение для проверки строки
    let re = regex::Regex::new(r"^[A-Za-z][0-9]$").unwrap();

    if !re.is_match(value) {
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

    let queue = Arc::new(Mutex::new(OrderQueue::new().await));
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
