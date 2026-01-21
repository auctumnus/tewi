use askama::Template;

use crate::models::threads::Thread;

#[derive(Template)]
#[template(path = "thread.html")]
pub struct ThreadTemplate {
    pub board_name: Option<String>,
    pub board_slugs: Vec<String>,
    pub thread: Thread,
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
