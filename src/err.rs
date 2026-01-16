use axum::{http::StatusCode, response::{IntoResponse, Response}};

#[derive(Debug, Clone)]
pub struct AppError {
    pub message: String,
    pub status_code: StatusCode,
}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError {
            message: format!("Database error: {}", err),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub type AppResult<T> = Result<T, AppError>;

pub fn internal_error(message: &str) -> AppError {
    AppError {
        message: message.to_string(),
        status_code: StatusCode::INTERNAL_SERVER_ERROR,
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.status_code, self.message).into_response()
    }
}