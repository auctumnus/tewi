use askama::Template;
use serde::Deserialize;

use crate::models::board_categories::BoardCategory;

#[derive(Debug, Deserialize)]
pub struct CreateBoardForm {
    pub name: String,
    pub slug: String,
    pub category_id: Option<String>,
}

pub struct CreateBoardValidationError {
    pub message: String,
}

#[derive(Template)]
#[template(path = "admin/create-board.html")]
pub struct CreateBoardTemplate {
    pub categories: Vec<BoardCategory>,
    pub validation: Option<CreateBoardValidationError>,
}
