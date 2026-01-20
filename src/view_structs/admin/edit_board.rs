use askama::Template;
use serde::Deserialize;

use crate::models::{board_categories::BoardCategory, boards::Board};

#[derive(Debug, Deserialize)]
pub struct EditBoardForm {
    pub name: String,
    pub slug: String,
    pub category_id: String,
}

pub struct EditBoardValidationError {
    pub message: String,
}

#[derive(Template)]
#[template(path = "admin/edit-board.html")]
pub struct EditBoardTemplate {
    pub validation: Option<EditBoardValidationError>,
    pub board: Board,
    pub categories: Vec<BoardCategory>,
}
