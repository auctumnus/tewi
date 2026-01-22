use askama::Template;
use serde::Deserialize;

use crate::models::board_categories::DBBoardCategory;

#[derive(Debug, Deserialize)]
pub struct QuickDeleteCategoryForm {
    pub name: String,
}

pub struct QuickDeleteCategoryValidationError {
    pub message: String,
}

#[derive(Template)]
#[template(path = "admin/categories.html")]
pub struct CategoriesTemplate {
    pub categories: Vec<DBBoardCategory>,
    pub validation: Option<QuickDeleteCategoryValidationError>,
}
