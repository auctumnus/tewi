use askama::Template;
use serde::Deserialize;
use uuid::Uuid;

use crate::models::boards::DbBoard;

#[derive(Template)]
#[template(path = "admin/create-attachment-policy.html")]
pub struct CreateAttachmentPolicyTemplate {
    pub validation: Option<String>,
    pub boards: Vec<DbBoard>,
    pub supported_mime_types: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AttachmentPoliciesForm {
    pub board: Uuid,
    pub enable_spoilers: bool,
    pub size_limit: i64,
    pub attachment_limit: i64,
    #[serde(default)]
    pub mime_types: Vec<String>,
}
