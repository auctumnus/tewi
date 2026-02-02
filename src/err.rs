use axum::{http::StatusCode, response::{IntoResponse, Response}};

use crate::parse_multipart::MultipartParseError;

#[derive(Debug, Clone)]
pub struct AppError {
    pub message: String,
    pub status_code: StatusCode,
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (status: {})", self.message, self.status_code)
    }
}

impl std::error::Error for AppError {}

impl From<sqlx::Error> for AppError {
    fn from(err: sqlx::Error) -> Self {
        AppError {
            message: format!("Database error: {}", err),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<sqlx::migrate::MigrateError> for AppError {
    fn from(err: sqlx::migrate::MigrateError) -> Self {
        AppError {
            message: format!("Migration error: {}", err),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<std::io::Error> for AppError {
    fn from(err: std::io::Error) -> Self {
        AppError {
            message: format!("IO error: {}", err),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl From<image::ImageError> for AppError {
    fn from(err: image::ImageError) -> Self {
        AppError {
            message: format!("Image error: {}", err),
            status_code: StatusCode::BAD_REQUEST,
        }
    }
}

impl From<MultipartParseError> for AppError {
    fn from(value: MultipartParseError) -> Self {
        AppError {
            message: format!("Multipart parse error: {:?}", value),
            status_code: StatusCode::BAD_REQUEST,
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

pub fn unauthorized(message: &str) -> AppError {
    AppError {
        message: message.to_string(),
        status_code: StatusCode::UNAUTHORIZED,
    }
}

pub fn banned(reason: &str) -> AppError {
    AppError {
        message: format!("IP address is banned: {}", reason),
        status_code: StatusCode::FORBIDDEN,
    }
}

pub fn invalid_credentials() -> AppError {
    AppError {
        message: "Invalid credentials".to_string(),
        status_code: StatusCode::UNAUTHORIZED,
    }
}

pub fn malformed(message: &str) -> AppError {
    AppError {
        message: message.to_string(),
        status_code: StatusCode::BAD_REQUEST,
    }
}

pub fn bad_request(message: &str) -> AppError {
    AppError {
        message: message.to_string(),
        status_code: StatusCode::BAD_REQUEST,
    }
}