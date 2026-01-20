use askama::Template;

use crate::{
    models::{boards::Board, threads::Thread},
    pagination::PaginatedResponse,
};

#[derive(Template)]
#[template(path = "page.html")]
pub struct BoardPageTemplate {
    pub board_name: Option<String>,
    pub board_slugs: Vec<String>,
    pub threads: PaginatedResponse<Thread>,
}

pub struct PostForm {
    pub name: String,
    pub title: String,
    pub attachments: String,
    pub content: String,
}
