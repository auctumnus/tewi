use askama::Template;

use crate::{
    extract_session::AdminSession,
    models::{admins::Admin, boards::Board, sessions::Session, threads::Thread},
    pagination::PaginatedResponse,
    parse_multipart::FormFileInfo,
};

#[derive(Template)]
#[template(path = "board-page.html")]
pub struct BoardPageTemplate {
    pub admin_session: Option<(Session, Admin)>,
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
