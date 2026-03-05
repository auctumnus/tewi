use askama::Template;

#[derive(Template)]
#[template(path = "status/errors/error-page.html")]
pub struct ErrorPageTemplate {
    pub title: Option<String>,
    pub message: Option<String>,
}
