use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct BanForm {
    pub reason: String,
    pub duration: String,
    pub timezone: String,
    pub also_delete: Option<bool>,
}
