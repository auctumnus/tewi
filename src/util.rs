use uuid::Uuid;

use crate::models::attachments::{attachment_path, thumbnail_path};


pub fn thumbnail_src(thumbnail_id: &Uuid) -> String {
    format!("/{}", thumbnail_path(thumbnail_id).to_string_lossy())
}

pub fn attachment_src(attachment_id: &Uuid) -> String {
    format!("/{}", attachment_path(attachment_id).to_string_lossy())
}