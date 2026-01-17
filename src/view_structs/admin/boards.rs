use askama::Template;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct QuickDeleteBoardForm {
    pub name: String,
    pub slug: String,
}

pub struct QuickDeleteBoardValidationError {
    pub message: String,
}

#[derive(Template)]
#[template(path = "admin/boards.html")]
pub struct BoardsTemplate {
    pub validation: Option<QuickDeleteBoardValidationError>,
}
