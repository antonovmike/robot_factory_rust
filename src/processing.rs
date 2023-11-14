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
use validator::Validate;
use validator_derive::Validate;

use crate::constants::{CHECK_INTERVAL, DATABASE_URL, SMTP_SENDER, SMTP_SERVER};
use crate::db::{validate_model_version, Database};
use crate::order::Order;

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct CurrentOrder {
    pub login: String,
    pub password: String,
    // Check that the model and version match the template [A-Za-z][0-9]
    #[validate(custom = "validate_model_version")]
    pub model: String,
    #[validate(custom = "validate_model_version")]
    pub version: String,
}

pub struct OrderQueue {
    pub orders: std::collections::VecDeque<CurrentOrder>,
    pool: Arc<Mutex<PgPool>>,
}

impl OrderQueue {
    pub async fn new() -> Self {
        let pool = match PgPool::connect(DATABASE_URL).await {
            Ok(pool) => pool,
            Err(err) => panic!("Failed to open database connection: {}", err),
        };
        // Wrap the connection in Arc and Mutex for secure access from different threads
        let pool = Arc::new(Mutex::new(pool));

        let orders = VecDeque::new();

        Self { orders, pool }
    }

    // Method for adding an order to the queue
    pub async fn enqueue(&mut self, order_current: CurrentOrder) {
        println!("Enqueue: {:?}", order_current);

        let db = Database::new().await.unwrap();
        let pool = self.pool.lock().await;
        let order = Order {
            customer_name: order_current.login.clone(),
            robot_model: order_current.model.clone(),
        };

        let result = Database::find_robot(&db, &order_current.model, &order_current.version).await;
        // Check that the result is not empty
        match result {
            Ok(0) => {
                println!("Product is out of stock");

                self.orders.push_back(order_current);
            }
            Ok(_) => {
                println!("Product is in stock");
                // Save completed order to "orders" table
                let customer_name: String =
                    sqlx::query_scalar("SELECT name FROM customers WHERE login = $1")
                        .bind(&order_current.login)
                        .fetch_one(&*pool)
                        .await
                        .unwrap();
                let robot_model = format!("{}-{}", &order_current.model, &order_current.version);
                order.add_order(customer_name, robot_model).await.unwrap();
            }
            Err(_) => {
                println!("product is out of stock");

                self.orders.push_back(order_current);
            }
        }
    }

    // Method for queue processing
    pub async fn process(&mut self) {
        loop {
            let pool = self.pool.lock().await;
            let db = Database::new().await.unwrap();

            // Vector for storing uncompleted orders
            let mut pending_orders = VecDeque::new();

            // Iterate over the order vector using the drain method, which moves elements from the vector
            for _ in 0..self.orders.len() {
                let order = self.orders.pop_front().unwrap();

                let customer_name: String =
                    sqlx::query_scalar("SELECT name FROM customers WHERE login = $1")
                        .bind(&order.login)
                        .fetch_one(&*pool)
                        .await
                        .unwrap();

                let result = Database::find_robot(&db, &order.model, &order.version).await;
                match result {
                    Ok(0) => {
                        pending_orders.push_back(order);
                    }
                    Ok(_) => {
                        println!("Hello {customer_name} product is available");
                        let db = Database::new().await.unwrap();

                        let message = format!(
                            "Добрый день, {}!\n\
                            Недавно вы интересовались нашим роботом модели {}, версии {}.\n\
                            Этот робот теперь в наличии. Если вам подходит этот вариант - пожалуйста, свяжитесь с нами",
                            customer_name, &order.model, &order.version
                        );

                        // This part of the code always returns an error
                        let (login, password) = (order.login, order.password);
                        let email_addr = db
                            .check_credentials(&login, &password)
                            .await
                            .unwrap()
                            .unwrap();
                        send_email(&email_addr, &message).expect("Failed to send email");

                        println!("{}", message);
                    }
                    Err(_) => {
                        pending_orders.push_back(order);
                    }
                }
            }
            // Replace the vector of orders with the vector of uncompleted orders
            self.orders = pending_orders;
            // Release access to the database connection
            drop(pool);

            sleep(Duration::from_secs(CHECK_INTERVAL)).await;
        }
    }
}

pub async fn order_robot(
    Json(order): Json<CurrentOrder>,
) -> Result<axum::http::StatusCode, axum::http::StatusCode> {
    if order.validate().is_err() {
        return Err(axum::http::StatusCode::BAD_REQUEST);
    }

    let queue = Arc::new(Mutex::new(OrderQueue::new().await));
    // Get a lock on the queue and add the order
    queue.lock().await.enqueue(order).await;
    // Запускаем задачу для обработки очереди
    let queue_clone = Arc::clone(&queue);
    tokio::spawn(async move {
        // Get a lock on the queue and call the process method
        queue_clone.lock().await.process().await;
    });

    // Checking to see if there are any orders in the queue
    if queue.lock().await.orders.is_empty() {
        // If queue is empty, return status code 200 (OK)
        Ok(axum::http::StatusCode::OK)
    } else {
        // If there are orders in the queue, return status code 404 (Not Found)
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
