use askama::Template;
use serde::Deserialize;

pub struct LoginValidationError {
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub username: String,
    pub password: String,
}

#[derive(Template)]
#[template(path = "admin/login.html")]
pub struct LoginTemplate {
    pub validation: Option<LoginValidationError>,
}
