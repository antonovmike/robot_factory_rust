use std::net::SocketAddr;

use axum::extract::Extension;
use axum::{
    Json,
    routing::{get, post},
    Router, Server,
};
use chrono::Local;
use sqlx::postgres::PgPool;

#[cfg(test)]
mod tests;

mod constants;
mod db;
mod order;
mod processing;
mod report;
mod robot;
mod structures;
mod user;

use constants::DATABASE_URL;
use db::Database;
use processing::order_robot;
use report::report_handler;
use robot::Robot;
use user::create_customer;

#[tokio::main]
async fn main() {
    amount_of_robots().await;

    let pool = PgPool::connect(DATABASE_URL).await.unwrap();
    let app = Router::new()
        .route("/robots/report", get(report_handler))
        .route("/create", post(move |Json(robot_data): Json<Robot>| async move {
            let robot = Robot {
                serial: robot_data.serial,
                model: robot_data.model,
                version: robot_data.version,
            };
            robot.create_robot().await
        }))
        .route("/remove", post(move |Json(robot_data): Json<Robot>| async move {
            let robot = Robot {
                serial: robot_data.serial,
                model: robot_data.model,
                version: robot_data.version,
            };
            robot.remove_robot().await
        }))
        .route("/robots/order", post(order_robot))
        .route("/user/create", post(create_customer))
        .layer(Extension(pool));

    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));

    Server::bind(&addr)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn amount_of_robots() {
    let db = Database::new().await.unwrap();

    match db.setup_database().await {
        Ok(_) => println!("Database setup completed successfully"),
        Err(e) => eprintln!("Database setup failed: {}", e),
    }

    let now = Local::now();
    let date = now.format("%Y-%m-%d %H:%M:%S").to_string();
    match db.get_robots_by_date(&date).await {
        Ok(count) => println!("Total amount of robots on {date} is {count}"),
        Err(e) => println!("Error: {}", e),
    }
}
