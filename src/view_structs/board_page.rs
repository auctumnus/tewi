use askama::Template;

use crate::{
    models::{boards::Board, threads::Thread},
    pagination::PaginatedResponse,
    parse_multipart::FormFileInfo,
};

#[derive(Template)]
#[template(path = "board-page.html")]
pub struct BoardPageTemplate {
    pub board: Board,
    pub board_slugs: Vec<String>,
    pub threads: PaginatedResponse<Thread>,
    pub form_route: String,
}

pub struct PostForm {
    pub name: String,
    pub title: String,
    pub attachments: Option<FormFileInfo>,
    pub content: String,
}
