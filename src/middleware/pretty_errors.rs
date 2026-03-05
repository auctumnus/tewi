use askama::Template;
use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{Html, IntoResponse, Response},
};

use crate::{err::AppError, view_structs};

pub async fn pretty_error_codes(request: Request, next: Next) -> Response {
    let response = next.run(request).await;

    if response.status().is_success() {
        return response;
    }

    match response.status() {
        StatusCode::NOT_FOUND => {
            let html =
                (view_structs::status::error::not_found::NotFoundTemplate { board_name: None })
                    .render()
                    .expect("Cant render the error template so just explode");

            return (StatusCode::NOT_FOUND, Html(html)).into_response();
        }
        StatusCode::UNAUTHORIZED => {
            let html =
                (view_structs::status::error::not_found::NotFoundTemplate { board_name: None })
                    .render()
                    .expect("Cant render the error template so just explode");

            return (StatusCode::NOT_FOUND, Html(html)).into_response();
        }
        StatusCode::INTERNAL_SERVER_ERROR => {
            let html =
                (view_structs::status::error::internal_server_error::InternalServerErrorTemplate {
                    board_name: None,
                })
                .render()
                .expect("Cant render the error template so just explode");

            return (StatusCode::INTERNAL_SERVER_ERROR, Html(html)).into_response();
        }
        _ => {
            let thing = response.extensions().get::<AppError>();
            return match thing {
                Some(err) => {
                    let html = (view_structs::status::error::error_page::ErrorPageTemplate {
                        message: Some(err.message.clone()),
                        title: None,
                    })
                    .render()
                    .expect("Cant render the error template so just explode");

                    (response.status(), Html(html)).into_response()
                }
                _ => response,
            };
        }
    };
}
