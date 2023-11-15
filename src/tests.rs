use super::*;
use crate::robot::Robot;

use axum::http;
use http::header::CONTENT_TYPE;

use axum::http::StatusCode;
use axum_test_helper::TestClient;
use chrono::Utc;
use sqlx::postgres::PgPool;

#[tokio::test]
async fn test_create_robot_valid() {
    let pool = PgPool::connect(DATABASE_URL).await.unwrap();
    let app = create_router(pool);
    let client = TestClient::new(app);

    let robot = Robot {
        serial: "T0".to_string(),
        model: "T0".to_string(),
        version: "T0".to_string(),
    };

    let res = client.post("/robots/create").json(&robot).send().await;
    assert_eq!(res.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn test_create_robot_invalid_serial() {
    let pool = PgPool::connect(DATABASE_URL).await.unwrap();
    let app = create_router(pool);
    let client = TestClient::new(app);

    let robot = Robot {
        serial: "".to_string(),
        model: "M1".to_string(),
        version: "V1".to_string(),
    };

    let res = client.post("/robots/create").json(&robot).send().await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_create_robot_invalid_model() {
    let pool = PgPool::connect(DATABASE_URL).await.unwrap();
    let app = create_router(pool);
    let client = TestClient::new(app);

    let robot = Robot {
        serial: "R1".to_string(),
        model: "123".to_string(),
        version: "V1".to_string(),
    };

    let res = client.post("/robots/create").json(&robot).send().await;
    assert_eq!(res.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn test_report_handler_success() {
    let pool = PgPool::connect(DATABASE_URL).await.unwrap();
    let app = create_router(pool);
    let client = TestClient::new(app);
    // Send a GET request to the report_handler
    let res = client.get("/robots/report").send().await;
    // Check the status of the response - it should be 200 OK
    assert_eq!(res.status(), StatusCode::OK);
    // The response type should be application/vnd.openxmlformats-officedocument.spreadsheetml.sheet
    assert_eq!(
        res.headers().get(CONTENT_TYPE).unwrap().to_str().unwrap(),
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
    );

    // The body of the response must contain the bytes of the Excel file
    // Use the bytes() method instead of body() to read the response body as a vector of bytes
    // Excel files start with bytes PK\x03\x04
    let body = res.bytes().await;
    assert!(body.starts_with(b"PK\x03\x04"));
}

#[tokio::test]
async fn test_remove_robot_valid() {
    let pool = PgPool::connect(DATABASE_URL).await.unwrap();
    let app = create_router(pool);
    let client = TestClient::new(app);

    // Creating a robot with valid values
    let robot = Robot {
        serial: "M10M1".to_string(),
        model: "M1".to_string(),
        version: "V1".to_string(),
    };

    // Add robot to Database
    let pool = PgPool::connect(DATABASE_URL).await.unwrap();
    let current_date = Utc::now().to_rfc3339();
    let statement = format!("INSERT INTO robots (serial, model, version, created) VALUES ($1, $2, $3, '{current_date}')");
    sqlx::query(&statement)
        .bind(&robot.serial)
        .bind(&robot.model)
        .bind(&robot.version)
        .execute(&pool)
        .await
        .unwrap();

    let res = client.post("/robots/remove").json(&robot).send().await;
    assert_eq!(res.status(), StatusCode::OK);
}

#[tokio::test]
async fn test_remove_robot_not_found() {
    let pool = PgPool::connect(DATABASE_URL).await.unwrap();
    let app = create_router(pool);
    let client = TestClient::new(app);

    // Trying to delete a robot that is not in the database
    let non_existent_robot = Robot {
        serial: "R99".to_string(),
        model: "M1".to_string(),
        version: "V1".to_string(),
    };
    let res = client
        .post("/robots/remove")
        .json(&non_existent_robot)
        .send()
        .await;
    assert_eq!(res.status(), StatusCode::NOT_FOUND);
}
