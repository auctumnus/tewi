use askama::Template;
use serde::Deserialize;
use uuid::Uuid;

use crate::models::{attachment_policies::AttachmentPolicy, boards::DbBoard};

#[derive(Template)]
#[template(path = "admin/edit-attachment-policy.html")]
pub struct EditAttachmentPolicyTemplate {
    pub validation: Option<String>,
    pub policy: AttachmentPolicy,
    pub boards: Vec<DbBoard>,
    pub supported_mime_types: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct AttachmentPoliciesForm {
    pub enable_spoilers: Option<bool>,
    pub size_limit: i64,
    pub attachment_limit: i64,
    #[serde(default)]
    pub mime_types: Vec<String>,
}
