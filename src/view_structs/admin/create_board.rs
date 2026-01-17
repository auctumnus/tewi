use askama::Template;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CreateBoardForm {
    pub name: String,
    pub slug: String,
}

pub struct CreateBoardValidationError {
    pub message: String,
}

#[derive(Template)]
#[template(path = "admin/create-board.html")]
pub struct CreateBoardTemplate {
    pub validation: Option<CreateBoardValidationError>,
}
