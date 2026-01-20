use askama::Template;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct CreateCategoryForm {
    pub name: String,
}

pub struct CreateCategoryValidationError {
    pub message: String,
}

#[derive(Template)]
#[template(path = "admin/create-category.html")]
pub struct CreateCategoryTemplate {
    pub validation: Option<CreateCategoryValidationError>,
}
