use askama::Template;

#[derive(Template)]
#[template(path = "status/errors/500.html")]
pub struct InternalServerErrorTemplate {}
