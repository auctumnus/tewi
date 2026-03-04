use askama::Template;
use uuid::Uuid;

#[derive(Template)]
#[template(path = "components/shiki-code-block.html")]
pub struct ShikiCodeBlockTemplate {
    pub block_id: Uuid,
    pub language: String,
    pub encoded_content: String,
    pub no_script_content: String,
}
