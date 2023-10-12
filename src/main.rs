use std::net::SocketAddr;

use axum::{
    routing::{get, post},
    Router, Server,
};
use axum_sqlite::*;
use chrono::Local;

#[cfg(test)]
mod tests;

mod constants;
mod create;
mod db;
mod order;
mod report;
mod structures;

use constants::DATABASE_NAME;
use db::get_robots_by_date;
use order::order_robot;
use report::report_handler;
use create::{create_robot, remove_robot};

#[tokio::main]
async fn main() {
    amount_of_robots().await;

    let app = Router::new()
        .route("/robots/report", get(report_handler))
        .route("/robots/create", post(create_robot))
        .route("/robots/order", post(order_robot))
        .route("/robots/remove", post(remove_robot))
        .layer(Database::new(DATABASE_NAME).unwrap());

    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));

    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn amount_of_robots() {
    let now = Local::now();
    let date = now.format("%Y-%m-%d %H:%M:%S").to_string();
    match get_robots_by_date(&date).await {
        Ok(count) => println!("Total amount of robots on {date} is {count}"),
        Err(e) => println!("Error: {}", e),
    }
}