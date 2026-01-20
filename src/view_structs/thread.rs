use askama::Template;

use crate::{models::threads::Thread, pagination::PaginatedResponse};

#[derive(Template)]
#[template(path = "thread.html")]
pub struct ThreadTemplate {
    pub board_name: Option<String>,
    pub board_slugs: Vec<String>,
    pub thread: Thread,
}

pub struct PostForm {
    pub name: String,
    pub title: String,
    pub attachments: String,
    pub content: String,
}
