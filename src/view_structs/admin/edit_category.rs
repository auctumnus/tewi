use askama::Template;
use serde::Deserialize;

use crate::models::board_categories::DBBoardCategory;

#[derive(Debug, Deserialize)]
pub struct EditCategoryForm {
    pub name: String,
}

pub struct EditCategoryValidationError {
    pub message: String,
}

#[derive(Template)]
#[template(path = "admin/edit-category.html")]
pub struct EditCategoryTemplate {
    pub validation: Option<EditCategoryValidationError>,
    pub category_info: Option<DBBoardCategory>,
}
