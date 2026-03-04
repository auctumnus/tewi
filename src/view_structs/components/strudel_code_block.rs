use askama::Template;
use uuid::Uuid;

#[derive(Template)]
#[template(path = "components/strudel-block.html")]
pub struct StrudelCodeBlockTemplate {
    pub block_id: Uuid,
    pub encoded_content: String,
    pub no_script_content: String,
}
