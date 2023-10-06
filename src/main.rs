use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::path::Path;

use axum::body::StreamBody;
use axum::http::{self, HeaderMap, HeaderValue};
use axum::{
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router, Server,
};
use axum_sqlite::*;
use http::header::CONTENT_TYPE;
use rusqlite::{Connection, Result};
use rust_xlsxwriter::Workbook;
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use validator::Validate;

#[cfg(test)]
mod tests;

mod db;
mod order;
use db::*;
use order::*;

const PATH_TO_XLSX: &str = "robots_report.xlsx";

struct _Customer {
    email: String,
}

struct _Order {
    customer: _Customer,
    robot_serial: String,
}

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

fn create_xlsx() -> Result<(), anyhow::Error> {
    // Check if the file exists and delete it if it does
    if fs::metadata(PATH_TO_XLSX).is_ok() {
        fs::remove_file(PATH_TO_XLSX).unwrap();
    }

    let conn = rusqlite::Connection::open(DATABASE_NAME).unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT model, version, COUNT(*) as count FROM robots 
            WHERE created >= date('now', '-7 day') GROUP BY model, version",
        )
        .unwrap();
    let robots_iter = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?, // model
                row.get::<_, String>(1)?, // version
                row.get::<_, i64>(2)?,    // count
            ))
        })
        .unwrap();
    let robots: Result<Vec<_>, _> = robots_iter.collect();
    let robots = robots.unwrap();

    // Create a HashMap where the key is the first letter of the model
    // and the value is a vector of tuples (model, version, count)
    let mut groups: HashMap<char, Vec<(String, String, i64)>> = HashMap::new();
    for (model, version, count) in robots {
        let first_char = model.chars().next().unwrap();
        groups
            .entry(first_char)
            .or_insert_with(Vec::new)
            .push((model, version, count));
    }

    // Create a new Excel file
    let mut workbook = Workbook::new();

    // Iterate over the groups and create a new sheet for each group
    for (key, value) in &groups {
        let sheet_name = format!("{}", key);
        let sheet = workbook.add_worksheet().set_name(sheet_name).unwrap();
        sheet.write_string(0, 0, "Model").unwrap();
        sheet.write_string(0, 1, "Version").unwrap();
        sheet.write_string(0, 2, "Quantity per week").unwrap();

        // Write the data for each group to the sheet
        for (i, (model, version, count)) in value.iter().enumerate() {
            sheet
                .write_string(i as u32 + 1, 0, model.to_string())
                .unwrap();
            sheet
                .write_string(i as u32 + 1, 1, version.to_string())
                .unwrap();
            sheet.write_number(i as u32 + 1, 2, *count as f64).unwrap();
        }
    }

    workbook.save(PATH_TO_XLSX).unwrap();
    Ok(())
}

async fn report_handler() -> Result<impl IntoResponse, (StatusCode, String)> {
    match tokio::task::spawn(async { create_xlsx() }).await {
        Ok(Ok(())) => (),
        Ok(Err(err)) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to create Excel file: {}", err),
            ))
        }
        Err(err) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to run blocking task: {}", err),
            ))
        }
    }

    let file = match File::open(PATH_TO_XLSX).await {
        Ok(file) => file,
        Err(err) => {
            return Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("File not found: {}", err),
            ))
        }
    };
    let stream = ReaderStream::new(file);
    let body = StreamBody::new(stream);
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_static(
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        ),
    );
    
    Ok((headers, body))
}

async fn create_robot(Json(robot): Json<Robot>) -> Result<StatusCode, StatusCode> {
    // Проверяем данные на валидность
    if robot.validate().is_err() {
        return Err(StatusCode::BAD_REQUEST);
    }
    // Открываем соединение с базой данных
    let conn = match Connection::open(Path::new("db.sqlite3")) {
        Ok(conn) => conn,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };
    // Создаем таблицу robots, если ее не существует
    match setup_database() {
        Ok(_) => (),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };
    // Формируем запрос на вставку данных о роботе
    let statement = format!(
        "INSERT INTO robots (serial, model, version, created) VALUES ('{}', '{}', '{}', '{}')",
        &robot.serial, &robot.model, &robot.version, &robot.created
    );
    // Выполняем запрос и возвращаем статус
    match conn.execute(&statement, []) {
        Ok(_) => Ok(StatusCode::CREATED),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
