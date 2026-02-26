use axum::{extract::Multipart, http::StatusCode};
use serde::{Deserialize, Serialize};

use crate::{
    err::{AppError, AppResult, bad_request, malformed},
    models::posts::AttachmentInfo,
    parse_multipart::read_chunks_until_done,
    view_structs,
};

pub struct PostInfo {
    pub title: String,
    pub name: String,
    pub content: String,
    pub attachments: Vec<AttachmentInfo>,
}

pub async fn multipart_to_post_info(mut multipart: Multipart) -> Result<PostInfo, AppError> {
    let mut title = String::new();
    let mut name = String::new();
    let mut content = String::new();
    let mut attachments = vec![];

    while let Some(field) = multipart.next_field().await.map_err(|_| AppError {
        message: "Failed to parse multipart form".to_owned(),
        status_code: StatusCode::BAD_REQUEST,
    })? {
        match field.name().unwrap_or("") {
            "title" => {
                title = field.text().await.map_err(|_| AppError {
                    message: "Failed to read title field".to_owned(),
                    status_code: StatusCode::BAD_REQUEST,
                })?;
            }
            "name" => {
                name = field.text().await.map_err(|_| AppError {
                    message: "Failed to read name field".to_owned(),
                    status_code: StatusCode::BAD_REQUEST,
                })?;
            }
            "content" => {
                content = field.text().await.map_err(|_| AppError {
                    message: "Failed to read content field".to_owned(),
                    status_code: StatusCode::BAD_REQUEST,
                })?;
            }
            "attachments" => {
                let content_type = field
                    .content_type()
                    .ok_or(bad_request("Missing content type for attachment"))?
                    .to_string();
                let filename = field
                    .file_name()
                    .ok_or(bad_request("Missing filename for attachment"))?
                    .to_string();

                let data = read_chunks_until_done(field).await.map_err(|_| AppError {
                    message: "Failed to read attachment data".to_owned(),
                    status_code: StatusCode::BAD_REQUEST,
                })?;
                if data.is_empty() {
                    continue;
                }

                attachments.push(AttachmentInfo {
                    data,
                    content_type,
                    filename,
                });
            }
            _ => {}
        }
    }

    Ok(PostInfo {
        title,
        name,
        content,
        attachments,
    })
}
