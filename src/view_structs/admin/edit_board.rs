use askama::Template;
use serde::Deserialize;

use crate::models::boards::Board;

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
    pub board_info: Option<Board>,
}
