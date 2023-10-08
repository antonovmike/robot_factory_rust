use super::*;
use crate::structures::Robot;

use axum::{routing::{get, post}, Router, http};
use http::header::CONTENT_TYPE;

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;
    use axum_test_helper::TestClient;

    #[tokio::test]
    async fn test_create_robot_valid() {
        let app = Router::new().route("/robots/create", post(create_robot));
        let client = TestClient::new(app);

        let robot = Robot {
            serial: "R1".to_string(),
            model: "M1".to_string(),
            version: "V1".to_string(),
            created: "2023-10-04".to_string(),
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
            created: "2023-10-04".to_string(),
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
            created: "2023-10-04".to_string(),
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
        // Тип содержимого ответа должен быть application/vnd.openxmlformats-officedocument.spreadsheetml.sheet
        assert_eq!(
            res.headers().get(CONTENT_TYPE).unwrap().to_str().unwrap(),
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
        );
        
        // Тело ответа должно содержать байты Excel-файла
        // Используем метод bytes() вместо body() для чтения тела ответа как вектора байтов
        let body = res.bytes().await;
        assert!(body.starts_with(b"PK\x03\x04")); // Excel-файлы начинаются с байтов PK\x03\x04
    }
}
