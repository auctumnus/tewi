use askama::Template;
use axum::body::Bytes;
use serde::Deserialize;

use crate::{
    models::{boards::Board, threads::Thread},
    pagination::PaginatedResponse,
};

#[derive(Template)]
#[template(path = "board-page.html")]
pub struct BoardPageTemplate {
    pub board_name: Option<String>,
    pub board_slugs: Vec<String>,
    pub threads: PaginatedResponse<Thread>,
    pub form_route: String,
}

pub struct PostForm {
    pub name: String,
    pub title: String,
    pub attachments: Bytes,
    pub content: String,
}
