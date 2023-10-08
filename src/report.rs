use std::collections::HashMap;
use std::fs;

use axum::body::StreamBody;
use axum::http::{self, HeaderMap, HeaderValue};
use axum::{
    http::StatusCode,
    response::IntoResponse,
};
use http::header::CONTENT_TYPE;
use rusqlite::Result;
use rust_xlsxwriter::{Workbook, Worksheet, XlsxError};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::constants::*;

fn create_xlsx() -> Result<(), anyhow::Error> {
    // Check if the file exists and delete it if it does
    if fs::metadata(PATH_TO_XLSX).is_ok() {
        fs::remove_file(PATH_TO_XLSX)?;
    }

    let conn = rusqlite::Connection::open(DATABASE_NAME)?;
    let robots = fetch_robots(&conn)?;

    let groups = group_robots_by_model(robots);
    create_excel_file(groups).unwrap();

    Ok(())
}

fn fetch_robots(conn: &rusqlite::Connection) -> rusqlite::Result<Vec<(String, String, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT model, version, COUNT(*) as count FROM robots 
        WHERE created >= date('now', '-7 day') GROUP BY model, version",
    )?;
    let robots_iter = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?, // model
            row.get::<_, String>(1)?, // version
            row.get::<_, i64>(2)?,    // count
        ))
    })?;
    robots_iter.collect()
}

fn group_robots_by_model(robots: Vec<(String, String, i64)>) -> HashMap<char, Vec<(String, String, i64)>> {
    let mut groups: HashMap<char, Vec<(String, String, i64)>> = HashMap::new();
    for (model, version, count) in robots {
        let first_char = model.chars().next().unwrap();
        groups
            .entry(first_char)
            .or_insert_with(Vec::new)
            .push((model, version, count));
    }
    groups
}

fn create_excel_file(groups: HashMap<char, Vec<(String, String, i64)>>) -> Result<(), Box<dyn std::error::Error>> {
    let mut workbook = Workbook::new();

    for (key, value) in &groups {
        let sheet_name = format!("{}", key);
        let sheet = workbook.add_worksheet().set_name(sheet_name)?;
        write_headers(sheet)?;
        write_data(sheet, value)?;
    }

    workbook.save(PATH_TO_XLSX).unwrap();
    Ok(())
}

fn write_headers(sheet: &mut Worksheet) -> Result<(), XlsxError> {
    for (i, header) in SHEET_HEADERS.iter().enumerate() {
        sheet.write_string(0, i as u16, header.to_string()).unwrap();
    }
    Ok(())
}

fn write_data(sheet: &mut Worksheet, data: &[(String, String, i64)]) -> Result<(), Box<dyn std::error::Error>> {
    for (i, (model, version, count)) in data.iter().enumerate() {
        sheet.write_string((i + 1) as u32, 0, model)?;
        sheet.write_string((i + 1) as u32, 1, version)?;
        sheet.write_number((i + 1) as u32, 2, *count as f64)?;
    }
    Ok(())
}


pub async fn report_handler() -> Result<impl IntoResponse, (StatusCode, String)> {
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
