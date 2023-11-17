use std::collections::HashMap;
use std::fs;

use axum::body::StreamBody;
use axum::http::{self, HeaderMap, HeaderValue};
use axum::{http::StatusCode, response::IntoResponse};
use http::header::CONTENT_TYPE;
use rust_xlsxwriter::{Workbook, Worksheet, XlsxError};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::constants::{PATH_TO_XLSX, SHEET_HEADERS};
use crate::db_pool::get_pool;

async fn create_xlsx() -> std::result::Result<(), anyhow::Error> {
    if fs::metadata(PATH_TO_XLSX).is_ok() {
        fs::remove_file(PATH_TO_XLSX)?;
    }

    let pool = get_pool().await.unwrap();
    let robots = fetch_robots(&pool).await?;

    let groups = robots
        .into_iter()
        .fold(HashMap::new(), |mut acc, (model, version, count)| {
            acc.entry(model.chars().next().unwrap())
                .or_insert_with(Vec::new)
                .push((model, version, count));
            acc
        });
    create_excel_file(groups).unwrap();

    Ok(())
}

async fn fetch_robots(pool: &sqlx::PgPool) -> sqlx::Result<Vec<(String, String, i64)>> {
    let robots: Vec<(String, String, i64)> = sqlx::query_as(
        "SELECT model, version, COUNT(*) as count FROM robots
        WHERE created >= current_date - interval '7 day' GROUP BY model, version",
    )
    .fetch_all(pool)
    .await?;

    Ok(robots)
}

fn create_excel_file(
    groups: HashMap<char, Vec<(String, String, i64)>>,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let mut workbook = Workbook::new();

    for (key, value) in &groups {
        let sheet_name = format!("{}", key);
        let sheet = workbook.add_worksheet().set_name(sheet_name)?;
        write_headers(sheet)?;
        write_data(sheet, value)?;
    }

    workbook.save(PATH_TO_XLSX)?;
    Ok(())
}

fn write_headers(sheet: &mut Worksheet) -> Result<(), XlsxError> {
    for (i, header) in SHEET_HEADERS.iter().enumerate() {
        sheet.write_string(0, i as u16, header.to_string())?;
    }
    Ok(())
}

fn write_data(
    sheet: &mut Worksheet,
    data: &[(String, String, i64)],
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    for (i, (model, version, count)) in data.iter().enumerate() {
        sheet.write_string((i + 1) as u32, 0, model)?;
        sheet.write_string((i + 1) as u32, 1, version)?;
        sheet.write_number((i + 1) as u32, 2, *count as f64)?;
    }
    Ok(())
}

pub async fn report_handler() -> std::result::Result<impl IntoResponse, (StatusCode, String)> {
    match create_xlsx().await {
        Ok(()) => {
            let file = File::open(PATH_TO_XLSX).await.map_err(|err| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("File not found: {}", err),
                )
            })?;
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
        Err(err) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to create Excel file: {}", err),
        )),
    }
}
