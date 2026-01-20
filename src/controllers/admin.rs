use std::str::FromStr;

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
        board_categories::{BoardCategoryRepository, EditBoardCategory},
        boards::{Board, BoardRepository, CreateBoard, EditBoard},
        sessions::SessionRepository,
    },
    view_structs::{self, admin::categories},
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
    State(s): State<AppState>,
    AdminSession(admin_session): AdminSession,
) -> Result<Html<String>, StatusCode> {
    let category_repo = BoardCategoryRepository::new(&s);
    let categories = category_repo
        .list_all()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match admin_session {
        Some(_) => {
            let html = (view_structs::admin::create_board::CreateBoardTemplate {
                categories,
                validation: None,
            })
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
            let category_repo = BoardCategoryRepository::new(&s);
            let board_repo = BoardRepository::new(&s);
            let board = board_repo
                .create(
                    admin,
                    CreateBoard {
                        description: "".to_string(),
                        name: payload.name,
                        slug: payload.slug,
                        category_id: match payload.category_id {
                            Some(category_id) => {
                                let parsed_uuid = Uuid::from_str(category_id.as_str())
                                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                                let category = category_repo
                                    .find_by_id(parsed_uuid)
                                    .await
                                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                                Some(category.id)
                            }
                            None => None,
                        },
                    },
                )
                .await
                .map_err(|_| StatusCode::UNAUTHORIZED)?;
            return Ok(Redirect::to(
                format!("/admin/boards/board/{}", board.slug).as_str(),
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
            let category_repo = BoardCategoryRepository::new(&s);

            if let Ok(board) = board_repo.find_by_slug(&path).await {
                let categories = category_repo
                    .list_all()
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                let html = (view_structs::admin::edit_board::EditBoardTemplate {
                    validation: None,
                    board: board,
                    categories,
                })
                .render()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                return Ok(Html(html));
            }
            let html =
                (view_structs::status::error::not_found::NotFoundTemplate { board_name: None })
                    .render()
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Html(html))
        }
        None => return Err(StatusCode::UNAUTHORIZED),
    }
}
pub async fn update_board(
    State(s): State<AppState>,
    Path(path): Path<String>,
    AdminSession(admin_session): AdminSession,
    Form(payload): Form<view_structs::admin::edit_board::EditBoardForm>,
) -> Result<Redirect, StatusCode> {
    match admin_session {
        Some((session, admin)) => {
            println!("desu");
            let board_repo = BoardRepository::new(&s);
            let category_repo = BoardCategoryRepository::new(&s);

            let board = board_repo
                .find_by_slug(&path)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let parsed_uuid = Uuid::from_str(payload.category_id.as_str())
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let category = category_repo
                .find_by_id(parsed_uuid)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let _ = board_repo
                .edit(
                    admin,
                    board.id,
                    EditBoard {
                        slug: Some(payload.slug),
                        name: Some(payload.name),
                        description: None,
                        category_id: Some(Some(category.id)),
                    },
                )
                .await;

            Ok(Redirect::to("/admin/boards"))
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}
pub async fn delete_board(
    State(s): State<AppState>,
    AdminSession(admin_session): AdminSession,
) -> Result<Html<String>, StatusCode> {
    match admin_session {
        Some(_) => {
            let html = (view_structs::admin::boards::BoardsTemplate {
                boards: vec![],
                validation: None,
            })
            .render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Html(html))
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}

pub async fn categories(
    AdminSession(admin_session): AdminSession,
    State(s): State<AppState>,
) -> Result<Html<String>, StatusCode> {
    match admin_session {
        Some(_admin_session) => {
            let category_repo = BoardCategoryRepository::new(&s);
            let categories = category_repo
                .list_all()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let db_categories: Vec<_> = categories
                .into_iter()
                .map(|c| crate::models::board_categories::DBBoardCategory {
                    id: c.id,
                    name: c.name,
                })
                .collect();
            let html = (view_structs::admin::categories::CategoriesTemplate {
                categories: db_categories,
                validation: None,
            })
            .render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Html(html))
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}

pub async fn display_create_category(
    State(_): State<AppState>,
    AdminSession(admin_session): AdminSession,
) -> Result<Html<String>, StatusCode> {
    match admin_session {
        Some(_) => {
            let html =
                (view_structs::admin::create_category::CreateCategoryTemplate { validation: None })
                    .render()
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Html(html))
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}

pub async fn create_category(
    State(s): State<AppState>,
    AdminSession(admin_session): AdminSession,
    Form(payload): Form<view_structs::admin::create_category::CreateCategoryForm>,
) -> Result<Redirect, StatusCode> {
    match admin_session {
        Some((_, admin)) => {
            let category_repo = BoardCategoryRepository::new(&s);
            let category = category_repo
                .create(admin, payload.name)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Redirect::to(
                format!("/admin/categories/category/{}", category.id).as_str(),
            ))
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}

pub async fn view_category(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    AdminSession(admin_session): AdminSession,
) -> Result<Html<String>, StatusCode> {
    match admin_session {
        Some(_admin_session) => {
            let category_repo = BoardCategoryRepository::new(&s);
            if let Ok(category) = category_repo.find_by_id(id).await {
                let html = (view_structs::admin::edit_category::EditCategoryTemplate {
                    validation: None,
                    category_info: Some(category),
                })
                .render()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                return Ok(Html(html));
            }
            Err(StatusCode::NOT_FOUND)
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}

pub async fn update_category(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    AdminSession(admin_session): AdminSession,
    Form(payload): Form<view_structs::admin::edit_category::EditCategoryForm>,
) -> Result<Redirect, StatusCode> {
    match admin_session {
        Some((_, admin)) => {
            let category_repo = BoardCategoryRepository::new(&s);
            category_repo
                .edit(
                    admin,
                    id,
                    EditBoardCategory {
                        name: Some(payload.name),
                    },
                )
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Redirect::to("/admin/categories"))
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}

pub async fn delete_category(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    AdminSession(admin_session): AdminSession,
) -> Result<Redirect, StatusCode> {
    match admin_session {
        Some((_, admin)) => {
            let category_repo = BoardCategoryRepository::new(&s);
            category_repo
                .delete(admin, id)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Redirect::to("/admin/categories"))
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}
