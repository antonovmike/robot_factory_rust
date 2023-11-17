use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::Extension;
use axum::{
    routing::{get, post},
    Json, Router, Server,
};
use chrono::Local;
use sqlx::postgres::PgPool;

#[cfg(test)]
mod tests;

mod constants;
mod db;
mod db_pool;
mod order;
mod processing;
mod report;
mod robot;
mod user;

use crate::db::Database;
use crate::db_pool::get_pool;
use processing::order_robot;
use report::report_handler;
use robot::Robot;
use user::create_customer;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    amount_of_robots().await?;

    let pool = get_pool().await?;
    let app = create_router(pool);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8000));

    Server::bind(&addr).serve(app.into_make_service()).await?;

    Ok(())
}

pub fn create_router(pool: Arc<PgPool>) -> Router<()> {
    let router: Router = Router::new()
        .route("/robots/report", get(report_handler))
        .route(
            "/robots/create",
            post(move |Json(robot_data): Json<Robot>| async move {
                let robot = Robot {
                    serial: robot_data.serial,
                    model: robot_data.model,
                    version: robot_data.version,
                };
                robot.create_robot().await
            }),
        )
        .route(
            "/robots/remove",
            post(move |Json(robot_data): Json<Robot>| async move {
                let robot = Robot {
                    serial: robot_data.serial,
                    model: robot_data.model,
                    version: robot_data.version,
                };
                robot.remove_robot().await
            }),
        )
        .route("/robots/order", post(order_robot))
        .route("/user/create", post(create_customer))
        .layer(Extension(pool));

    router
}

async fn amount_of_robots() -> anyhow::Result<()> {
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

    Ok(())
}
