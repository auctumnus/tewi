use askama::Template;

use crate::models::{boards::Board, threads::Thread};

mod filters {
    use uuid::Uuid;

    #[askama::filter_fn]
    pub fn thumbnail_url(value: &Uuid, _: &dyn askama::Values) -> askama::Result<String> {
        Ok(crate::models::attachments::thumbnail_path(value.clone())
            .to_string_lossy()
            .to_string())
    }

    #[askama::filter_fn]
    pub fn attachment_url(value: &Uuid, _: &dyn askama::Values) -> askama::Result<String> {
        Ok(crate::models::attachments::attachment_path(value.clone())
            .to_string_lossy()
            .to_string())
    }
}

#[derive(Template)]
#[template(path = "thread.html")]
pub struct ThreadTemplate {
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
