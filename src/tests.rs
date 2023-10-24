use super::*;
use crate::structures::Robot;

use axum::{
    http,
    routing::{get, post},
    Router,
};
use http::header::CONTENT_TYPE;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum_test_helper::TestClient;
    use chrono::Utc;
    use sqlx::postgres::PgPool;

    #[tokio::test]
    async fn test_create_robot_valid() {
        let app = Router::new().route("/robots/create", post(create_robot));
        let client = TestClient::new(app);

        let robot = Robot {
            serial: "R1".to_string(),
            model: "M1".to_string(),
            version: "V1".to_string(),
        };

        let res = client.post("/robots/create").json(&robot).send().await;
        assert_eq!(res.status(), StatusCode::CREATED);
    }

    #[tokio::test]
    async fn test_create_robot_invalid_serial() {
        let app = Router::new().route("/robots/create", post(create_robot));
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
        let app = Router::new().route("/robots/create", post(create_robot));
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
        // Маршрутизатор с обработчиком report_handler и тестовый клиент
        let app = Router::new().route("/robots/report", get(report_handler));
        let client = TestClient::new(app);
        // Отправляем GET-запрос к обработчику report_handler
        let res = client.get("/robots/report").send().await;
        // Проверяем статус ответа - должен быть 200 OK
        assert_eq!(res.status(), StatusCode::OK);
        // Тип ответа должен быть application/vnd.openxmlformats-officedocument.spreadsheetml.sheet
        assert_eq!(
            res.headers().get(CONTENT_TYPE).unwrap().to_str().unwrap(),
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        );

        // Тело ответа должно содержать байты Excel-файла
        // Используем метод bytes() вместо body() для чтения тела ответа как вектора байтов
        // Excel-файлы начинаются с байтов PK\x03\x04
        let body = res.bytes().await;
        assert!(body.starts_with(b"PK\x03\x04"));
    }

    #[tokio::test]
    async fn test_remove_robot_valid() {
        let app = Router::new()
            .route("/robots/create", post(create_robot))
            .route("/robots/remove", post(remove_robot));
        let client = TestClient::new(app);

        // Создаем робота с допустимыми значениями
        let robot = Robot {
            serial: "M10M1".to_string(),
            model: "M1".to_string(),
            version: "V1".to_string(),
        };

        // Добавляем робота в базу данных
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
        let app = Router::new().route("/robots/remove", post(remove_robot));
        let client = TestClient::new(app);

        // Пытаемся удалить робота, которого нет в базе данных
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
}
