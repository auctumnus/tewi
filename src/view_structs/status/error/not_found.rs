use askama::Template;

#[derive(Template)]
#[template(path = "status/errors/404.html")]
pub struct NotFoundTemplate {}
