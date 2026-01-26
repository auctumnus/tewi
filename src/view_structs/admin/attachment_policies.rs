use askama::Template;

use crate::models::attachment_policies::AttachmentPolicy;

#[derive(Template)]
#[template(path = "admin/attachment-policies.html")]
pub struct AttachmentPoliciesTemplate {
    pub attachment_policies: Vec<AttachmentPolicy>,
}
