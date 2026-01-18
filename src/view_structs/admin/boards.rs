use askama::Template;
use serde::Deserialize;

use crate::models::boards::Board;

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
    pub boards: Vec<Board>,
    pub validation: Option<QuickDeleteBoardValidationError>,
}
