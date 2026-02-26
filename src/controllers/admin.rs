use std::str::FromStr;

use askama::Template;
use axum::{
    extract::{Form, Path, State},
    http::StatusCode,
    response::{Html, Redirect},
};
use axum_extra::extract::{CookieJar, cookie::Cookie};
use uuid::Uuid;

use crate::{
    AppState,
    err::{AppError, AppResult},
    extract_session::{self, AdminSession},
    models::{
        attachment_policies::{
            AttachmentPolicy, AttachmentPolicyRepository, CreateAttachmentPolicy,
            EditAttachmentPolicy, SUPPORTED_MIME_TYPES,
        },
        bans::BanRepository,
        board_categories::{BoardCategoryRepository, EditBoardCategory},
        boards::{BoardRepository, CreateBoard, EditBoard},
        sessions::SessionRepository,
    },
    view_structs::{self},
};

pub async fn login_page(State(_s): State<AppState>) -> Result<Html<String>, StatusCode> {
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
        let mut cookie = Cookie::new(extract_session::SESSION_COOKIE_NAME, session.token);
        cookie.set_path("/");
        return Ok((jar.add(cookie), Redirect::to("/admin/boards")));
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
    Path(path): Path<Uuid>,
    AdminSession(admin_session): AdminSession,
) -> Result<Html<String>, StatusCode> {
    match admin_session {
        Some(_admin_session) => {
            let board_repo = BoardRepository::new(&s);
            let category_repo = BoardCategoryRepository::new(&s);

            if let Ok(board) = board_repo.find_by_id(path).await {
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
    Path(path): Path<Uuid>,
    AdminSession(admin_session): AdminSession,
    Form(payload): Form<view_structs::admin::edit_board::EditBoardForm>,
) -> Result<Redirect, StatusCode> {
    match admin_session {
        Some((_session, admin)) => {
            let board_repo = BoardRepository::new(&s);
            let category_repo = BoardCategoryRepository::new(&s);

            let board = board_repo
                .find_by_id(path)
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
    State(_s): State<AppState>,
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

pub async fn bans(
    AdminSession(admin_session): AdminSession,
    State(s): State<AppState>,
) -> Result<Html<String>, StatusCode> {
    match admin_session {
        Some(_) => {
            let ban_repo = BanRepository::new(&s);
            let raw_bans = ban_repo
                .list_all()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let mut bans = Vec::with_capacity(raw_bans.len());
            for ban in raw_bans {
                let entry = ban_repo
                    .materialize(ban)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
                bans.push(entry);
            }

            let html = (view_structs::admin::bans::BansTemplate { bans })
                .render()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Html(html))
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}

pub async fn attachment_policies(
    AdminSession(admin_session): AdminSession,
    State(s): State<AppState>,
) -> Result<Html<String>, StatusCode> {
    match admin_session {
        Some(_) => {
            let attachment_policy_repo = AttachmentPolicyRepository::new(&s);
            let policies = attachment_policy_repo
                .list_all()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let mut policies_materialized = Vec::<AttachmentPolicy>::new();
            for policy in policies {
                let asdf = attachment_policy_repo.materialize(policy).await;
                match asdf {
                    Ok(asdf) => policies_materialized.push(asdf),
                    Err(_) => continue,
                };
            }
            let html = (view_structs::admin::attachment_policies::AttachmentPoliciesTemplate {
                attachment_policies: policies_materialized,
            })
            .render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Html(html))
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}
pub async fn show_create_attachment_policies(
    AdminSession(admin_session): AdminSession,
    State(s): State<AppState>,
) -> Result<Html<String>, StatusCode> {
    match admin_session {
        Some(_) => {
            let board_repository = BoardRepository::new(&s);
            let boards = board_repository
                .list_all()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let html =
                (view_structs::admin::create_attachment_policy::CreateAttachmentPolicyTemplate {
                    validation: None,
                    boards: boards,
                    supported_mime_types: SUPPORTED_MIME_TYPES
                        .iter()
                        .map(|mime| mime.to_string())
                        .collect(),
                })
                .render()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Html(html))
        }
        None => Err(StatusCode::UNAUTHORIZED),
    }
}
pub async fn create_attachment_policies(
    AdminSession(admin_session): AdminSession,
    State(s): State<AppState>,
    axum_extra::extract::Form(payload): axum_extra::extract::Form<
        view_structs::admin::create_attachment_policy::AttachmentPoliciesForm,
    >,
) -> AppResult<Redirect> {
    match admin_session {
        Some((_, admin)) => {
            let board_repo = BoardRepository::new(&s);
            let board = board_repo.find_by_id(payload.board).await?;
            let policy_repo = AttachmentPolicyRepository::new(&s);

            policy_repo
                .create(
                    admin,
                    CreateAttachmentPolicy {
                        board_id: board.id,
                        mime_types: payload.mime_types,
                        enable_spoilers: payload.enable_spoilers,
                        size_limit: payload.size_limit,
                    },
                )
                .await?;

            Ok(Redirect::to("/admin/attachment-policies"))
        }
        None => Err(AppError {
            message: "Not an admin".to_string(),
            status_code: StatusCode::UNAUTHORIZED,
        }),
    }
}
pub async fn show_edit_attachment_policies(
    AdminSession(admin_session): AdminSession,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Html<String>> {
    match admin_session {
        Some(_) => {
            let board_repository = BoardRepository::new(&s);
            let boards = board_repository.list_all().await?;

            let policy_repo = AttachmentPolicyRepository::new(&s);
            let policy = policy_repo.find_by_id(id).await?;
            let policy = policy_repo.materialize(policy).await?;

            let html =
                (view_structs::admin::edit_attachment_policy::EditAttachmentPolicyTemplate {
                    validation: None,
                    policy,
                    boards,
                    supported_mime_types: SUPPORTED_MIME_TYPES
                        .iter()
                        .map(|mime| mime.to_string())
                        .collect(),
                })
                .render()
                .map_err(|_| AppError {
                    message: "Rendering Error".to_string(),
                    status_code: StatusCode::INTERNAL_SERVER_ERROR,
                })?;
            Ok(Html(html))
        }
        None => Err(AppError {
            message: "Not an admin".to_string(),
            status_code: StatusCode::UNAUTHORIZED,
        }),
    }
}
pub async fn edit_attachment_policies(
    AdminSession(admin_session): AdminSession,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    axum_extra::extract::Form(payload): axum_extra::extract::Form<
        view_structs::admin::edit_attachment_policy::AttachmentPoliciesForm,
    >,
) -> AppResult<Redirect> {
    match admin_session {
        Some((_, admin)) => {
            let policy_repo = AttachmentPolicyRepository::new(&s);
            let policy = policy_repo.find_by_id(id).await?;

            policy_repo
                .edit(
                    admin,
                    policy.id,
                    EditAttachmentPolicy {
                        board_id: None,
                        mime_types: Some(payload.mime_types),
                        enable_spoilers: payload.enable_spoilers.or(Some(false)),
                        size_limit: Some(payload.size_limit),
                    },
                )
                .await?;

            Ok(Redirect::to("/admin/attachment-policies"))
        }
        None => Err(AppError {
            message: "Not an admin".to_string(),
            status_code: StatusCode::UNAUTHORIZED,
        }),
    }
}
pub async fn delete_attachment_policies(
    AdminSession(admin_session): AdminSession,
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Redirect> {
    match admin_session {
        Some((_, admin)) => {
            let policy_repo = AttachmentPolicyRepository::new(&s);
            let policy = policy_repo.find_by_id(id).await?;

            policy_repo.delete(admin, policy.id).await?;

            Ok(Redirect::to("/admin/attachment-policies"))
        }
        None => Err(AppError {
            message: "Not an admin".to_string(),
            status_code: StatusCode::UNAUTHORIZED,
        }),
    }
}
