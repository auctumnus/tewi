use askama::Template;

use crate::{
    extract_session::AdminSession,
    models::{admins::Admin, boards::Board, sessions::Session, threads::Thread},
};

#[derive(Template)]
#[template(path = "thread.html")]
pub struct ThreadTemplate {
    pub admin_session: Option<(Session, Admin)>,
    pub board: Board,
    pub board_slugs: Vec<String>,
    pub thread: Thread,
    pub form_route: String,
}

pub struct PostFormTextFields {
    pub name: String,
    pub title: String,
    pub content: String,
}
pub struct PostFormFiles {
    pub attachments: String,
}

pub struct PostForm(PostFormTextFields, PostFormFiles);
