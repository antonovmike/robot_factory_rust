use std::net::SocketAddr;

use axum::{
    routing::{get, post},
    Router, Server,
};
use axum_sqlite::*;

#[cfg(test)]
mod tests;

mod constants;
mod create;
mod db;
mod order;
mod report;

use constants::DATABASE_NAME;
use db::*;
use order::*;
use report::*;
use create::*;

// struct _Customer {
//     email: String,
// }

// struct _Order {
//     customer: _Customer,
//     robot_serial: String,
// }

#[tokio::main]
async fn main() {
    let current_day = "2023-10-06 12:17:22";
    let stats = get_robots_by_date(current_day).unwrap();
    println!("Total amount of robots on {current_day} is {stats}");

    // Создаем маршрутизатор
    let app = Router::new()
        .route("/robots/report", get(report_handler))
        .route("/robots/create", post(create_robot))
        .route("/robots/order", post(order_robot))
        .layer(Database::new(DATABASE_NAME).unwrap());

    // Запускаем сервер на локальном адресе
    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));

    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}
