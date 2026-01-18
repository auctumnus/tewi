use axum::{extract::Request, middleware::Next, response::Response};

use crate::extract_session::AdminSession;

pub async fn verify_auth(
    request: Request,
    //AdminSession(admin_session): AdminSession,
    next: Next,
) -> Response {
    /*  match admin_session {
           Some(admin_session) => {
               let response = next.run(request).await;

               response
           }
           None => {
               panic!("lets all die") //axum::Error::new(StatusCode::UNAUTHORIZED),
           }
       }
    */
    // do something with `response`...

    let response = next.run(request).await;

    response
}
