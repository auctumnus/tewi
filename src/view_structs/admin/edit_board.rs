use askama::Template;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct EditBoardForm {
    pub name: String,
    pub slug: String,
}

pub struct EditBoardValidationError {
    pub message: String,
}

#[derive(Template)]
#[template(path = "admin/edit-board.html")]
pub struct EditBoardTemplate {
    pub validation: Option<EditBoardValidationError>,
}
