use askama::Template;

use crate::models::bans::BanListEntry;

#[derive(Template)]
#[template(path = "admin/bans.html")]
pub struct BansTemplate {
    pub bans: Vec<BanListEntry>,
}
