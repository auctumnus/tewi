use askama::Template;
use axum::{
    Error,
    extract::{Form, MatchedPath, Path, State},
    http::StatusCode,
    response::{Html, Redirect},
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use uuid::Uuid;

use crate::{
    AppState, auth,
    extract_session::{self, AdminSession},
    models::{
        admins::AdminRepository,
        board_categories::BoardCategoryRepository,
        boards::{Board, BoardRepository, CreateBoard},
        sessions::SessionRepository,
    },
    view_structs,
};

pub async fn login_page(State(s): State<AppState>) -> Result<Html<String>, StatusCode> {
    let html = (view_structs::admin::login::LoginTemplate { validation: None })
        .render()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Html(html))
}
pub async fn login(
    jar: CookieJar,
    State(s): State<AppState>,
    Form(payload): Form<view_structs::admin::login::LoginForm>,
) -> Result<(CookieJar, Redirect), StatusCode> {
    let sessions_repo = SessionRepository::new(&s);

    if let Ok(session) = sessions_repo
        .create(&payload.username.as_str(), &payload.password.as_str())
        .await
    {
        return Ok((
            jar.add(Cookie::new(
                extract_session::SESSION_COOKIE_NAME,
                session.token,
            )),
            Redirect::to("/admin/boards"),
        ));
    }

    Err(StatusCode::UNAUTHORIZED)
}

pub async fn logout(
    State(s): State<AppState>,
    AdminSession(admin): AdminSession,
) -> Result<Html<String>, StatusCode> {
    let sessions_repo = SessionRepository::new(&s);
    match admin {
        Some(admin) => {
            if let Ok(_session) = sessions_repo.delete_by_token(&admin.0.token).await {
                let html = (view_structs::admin::login::LoginTemplate { validation: None })
                    .render()
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                return Ok(Html(html));
            } else {
                return Err(StatusCode::UNAUTHORIZED);
            }
        }
        None => return Err(StatusCode::UNAUTHORIZED),
    }
}

pub async fn boards(
    AdminSession(admin_session): AdminSession,
    State(s): State<AppState>,
) -> Result<Html<String>, StatusCode> {
    match admin_session {
        Some(_admin_session) => {
            let board_repo = BoardRepository::new(&s);
            let boards = board_repo
                .list_all()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let html = (view_structs::admin::boards::BoardsTemplate {
                boards,
                validation: None,
            })
            .render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Html(html))
        }
        None => return Err(StatusCode::UNAUTHORIZED),
    }
}
pub async fn display_create_board(
    State(_): State<AppState>,
    AdminSession(admin_session): AdminSession,
) -> Result<Html<String>, StatusCode> {
    match admin_session {
        Some(_) => {
            let html =
                (view_structs::admin::create_board::CreateBoardTemplate { validation: None })
                    .render()
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Html(html))
        }
        None => return Err(StatusCode::UNAUTHORIZED),
    }
}
pub async fn create_board(
    State(s): State<AppState>,
    AdminSession(admin_session): AdminSession,
    Form(payload): Form<view_structs::admin::create_board::CreateBoardForm>,
) -> Result<Redirect, StatusCode> {
    match admin_session {
        Some((_, admin)) => {
            let board_repo = BoardRepository::new(&s);
            let board = board_repo
                .create(
                    admin,
                    CreateBoard {
                        category_id: None,
                        description: "".to_string(),
                        name: payload.name,
                        slug: payload.slug,
                    },
                )
                .await
                .map_err(|_| StatusCode::UNAUTHORIZED)?;
            return Ok(Redirect::to(
                format!("/admin/boards/{}", board.slug).as_str(),
            ));
        }
        None => return Err(StatusCode::UNAUTHORIZED),
    }
}

pub async fn view_board(
    State(s): State<AppState>,
    Path(path): Path<String>,
    AdminSession(admin_session): AdminSession,
) -> Result<Html<String>, StatusCode> {
    match admin_session {
        Some(admin_session) => {
            let board_repo = BoardRepository::new(&s);
            if let Ok(board) = board_repo.find_by_slug(&path).await {
                let html = (view_structs::admin::edit_board::EditBoardTemplate {
                    validation: None,
                    board_info: None,
                })
                .render()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                return Ok(Html(html));
            }
            let html =
                (view_structs::admin::create_board::CreateBoardTemplate { validation: None })
                    .render()
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Html(html))
        }
        None => return Err(StatusCode::UNAUTHORIZED),
    }
}
pub async fn update_board(State(s): State<AppState>) -> Result<Html<String>, StatusCode> {
    let html = (view_structs::admin::edit_board::EditBoardTemplate {
        validation: None,
        board_info: None,
    })
    .render()
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Html(html))
}
pub async fn delete_board(State(s): State<AppState>) -> Result<Html<String>, StatusCode> {
    let html = (view_structs::admin::boards::BoardsTemplate {
        boards: vec![],
        validation: None,
    })
    .render()
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Html(html))
}
