use askama::Template;

use crate::{
    models::{boards::Board, threads::Thread},
    pagination::PaginatedResponse,
    parse_multipart::FormFileInfo,
};

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
    pub attachments: FormFileInfo,
    pub content: String,
}
