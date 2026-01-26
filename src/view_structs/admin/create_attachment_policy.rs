use askama::Template;
use serde::{Deserialize, Deserializer};
use uuid::Uuid;

use crate::models::boards::Board;

#[derive(Template)]
#[template(path = "admin/create-attachment-policy.html")]
pub struct CreateAttachmentPolicyTemplate {
    pub validation: Option<String>,
    pub boards: Vec<Board>,
    pub supported_mime_types: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AttachmentPoliciesForm {
    pub board: Uuid,
    pub enable_spoilers: bool,
    pub size_limit: i64,
    #[serde(default)]
    pub mime_types: Vec<String>,
}
