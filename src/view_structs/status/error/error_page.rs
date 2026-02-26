use askama::Template;

#[derive(Template)]
#[template(path = "status/errors/error-page.html")]
pub struct ErrorPageTemplate {
    pub message: Option<String>,
    pub info: Option<String>,
}
