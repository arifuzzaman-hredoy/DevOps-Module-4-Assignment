use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct ErrorDetail {
    code: String,
    message: String,
}

#[derive(Debug, Serialize)]
struct ErrorBody {
    error: ErrorDetail,
}

#[derive(Debug)]
pub struct AppError {
    status: StatusCode,
    code: String,
    message: String,
}

impl AppError {
    pub fn bad_request(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn validation(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self::bad_request(code, message)
    }

    pub fn not_found(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::NOT_FOUND,
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn unauthorized() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "UNAUTHENTICATED".to_string(),
            message: "Authentication is required".to_string(),
        }
    }

    pub fn authentication_failed() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            code: "AUTHENTICATION_FAILED".to_string(),
            message: "Email or password is incorrect".to_string(),
        }
    }

    pub fn forbidden(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn conflict(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::CONFLICT,
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn too_many_requests(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::TOO_MANY_REQUESTS,
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: "INTERNAL_SERVER_ERROR".to_string(),
            message: message.into(),
        }
    }

    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: "SERVICE_UNAVAILABLE".to_string(),
            message: message.into(),
        }
    }

    pub fn method_not_allowed() -> Self {
        Self {
            status: StatusCode::METHOD_NOT_ALLOWED,
            code: "METHOD_NOT_ALLOWED".to_string(),
            message: "Method not allowed".to_string(),
        }
    }

    pub fn payload_too_large() -> Self {
        Self {
            status: StatusCode::PAYLOAD_TOO_LARGE,
            code: "PAYLOAD_TOO_LARGE".to_string(),
            message: "Request body exceeds the allowed size".to_string(),
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let payload = ErrorBody {
            error: ErrorDetail {
                code: self.code,
                message: self.message,
            },
        };

        (self.status, Json(payload)).into_response()
    }
}

impl From<axum::extract::rejection::JsonRejection> for AppError {
    fn from(rejection: axum::extract::rejection::JsonRejection) -> Self {
        if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
            Self::payload_too_large()
        } else {
            Self::bad_request("INVALID_JSON", "Invalid JSON payload")
        }
    }
}

impl From<axum::extract::rejection::QueryRejection> for AppError {
    fn from(_: axum::extract::rejection::QueryRejection) -> Self {
        Self::bad_request("INVALID_QUERY", "Invalid query parameters")
    }
}

impl From<sqlx::Error> for AppError {
    fn from(value: sqlx::Error) -> Self {
        tracing::error!("Database error: {}", value);
        Self::internal("Database error")
    }
}

impl From<std::io::Error> for AppError {
    fn from(value: std::io::Error) -> Self {
        tracing::error!("I/O error: {}", value);
        Self::internal("I/O error")
    }
}

#[cfg(test)]
mod tests {
    use super::AppError;
    use axum::{body::to_bytes, http::StatusCode, response::IntoResponse};

    #[tokio::test]
    async fn bad_request_uses_standard_json_error_envelope() {
        let response =
            AppError::bad_request("INVALID_QUERY", "Invalid query parameters").into_response();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"]["code"], "INVALID_QUERY");
        assert_eq!(body["error"]["message"], "Invalid query parameters");
    }

    #[tokio::test]
    async fn method_not_allowed_uses_standard_json_error_envelope() {
        let response = AppError::method_not_allowed().into_response();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

        let body = to_bytes(response.into_body(), 1024).await.unwrap();
        let body: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(body["error"]["code"], "METHOD_NOT_ALLOWED");
    }
}
