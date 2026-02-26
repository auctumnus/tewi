use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct DeleteForm {
    pub action: String,
    pub remove_content: Option<bool>,
    pub remove_attachment: Option<bool>,
}
