use askama::Template;
use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{Html, IntoResponse, Response},
};

use crate::view_structs;

pub async fn pretty_error_codes(request: Request, next: Next) -> Response {
    let response = next.run(request).await;

    match response.status() {
        StatusCode::NOT_FOUND => {
            let html =
                (view_structs::status::error::not_found::NotFoundTemplate { board_name: None })
                    .render()
                    .expect("Cant render the error template so just explode");

            // this feels hacky but i couldn't find another way.
            //  grab the headers and status code from the original
            //  response and make a new response with an html body
            let parts = response.into_parts();
            let parts_two = Html(html).into_response().into_parts();

            let constructed_response = Response::from_parts(parts.0, parts_two.1);
            return constructed_response;
        }
        _ => return response,
    };
}
