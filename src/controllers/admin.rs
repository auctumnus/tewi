use askama::Template;
use axum::{
    extract::{Form, State},
    http::StatusCode,
    response::Html,
};

use crate::{AppState, auth, models::admins::AdminRepository, view_structs};

pub async fn login_page(State(s): State<AppState>) -> Result<Html<String>, StatusCode> {
    let html = (view_structs::admin::login::LoginTemplate { validation: None })
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Html(html))
}
pub async fn login(
    State(s): State<AppState>,
    Form(payload): Form<view_structs::admin::login::LoginForm>,
) -> Result<Html<String>, StatusCode> {
    let admin_repo = AdminRepository::new(&s);
    if let Ok(admin) = admin_repo.find_by_name(payload.username.as_str()).await {
        match auth::verify(payload.password.as_str(), &admin.password_hash) {
            Ok(is_valid) => {
                if is_valid {
                    let html = (view_structs::admin::login::LoginTemplate { validation: None })
                        .render()
                        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                    return Ok(Html(html));
                }
            }
            Err(_) => {}
        }
    }
    let html = (view_structs::admin::login::LoginTemplate {
        validation: Some(view_structs::admin::login::LoginValidationError {
            message: "Username or Password invalid".to_string(),
        }),
    })
    .render()
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Html(html))
}

pub async fn boards(State(s): State<AppState>) -> Result<Html<String>, StatusCode> {
    let html = (view_structs::admin::boards::BoardsTemplate { validation: None })
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Html(html))
}
pub async fn display_create_board(State(s): State<AppState>) -> Result<Html<String>, StatusCode> {
    let html = (view_structs::admin::create_board::CreateBoardTemplate { validation: None })
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Html(html))
}
pub async fn create_board(State(s): State<AppState>) -> Result<Html<String>, StatusCode> {
    let html = (view_structs::admin::create_board::CreateBoardTemplate { validation: None })
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Html(html))
}

pub async fn view_board(State(s): State<AppState>) -> Result<Html<String>, StatusCode> {
    let html = (view_structs::admin::edit_board::EditBoardTemplate { validation: None })
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Html(html))
}
pub async fn update_board(State(s): State<AppState>) -> Result<Html<String>, StatusCode> {
    let html = (view_structs::admin::edit_board::EditBoardTemplate { validation: None })
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Html(html))
}
pub async fn delete_board(State(s): State<AppState>) -> Result<Html<String>, StatusCode> {
    let html = (view_structs::admin::boards::BoardsTemplate { validation: None })
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Html(html))
}
