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
    err::{AppError, AppResult, internal_error, unauthorized},
    extract_session::{self, AdminSession},
    models::{
        attachment_policies::{
            AttachmentPolicy, AttachmentPolicyRepository, CreateAttachmentPolicy,
            EditAttachmentPolicy, SUPPORTED_MIME_TYPES,
        },
        bans::BanRepository,
        board_categories::{BoardCategoryRepository, EditBoardCategory},
        boards::{Board, BoardRepository, CreateBoard, EditBoard},
        sessions::SessionRepository,
    },
    view_structs::{self},
};

pub async fn login_page(State(_s): State<AppState>) -> AppResult<Html<String>> {
    let html = (view_structs::admin::login::LoginTemplate { validation: None })
        .render()
        .map_err(|_| internal_error("Template render failed"))?;
    Ok(Html(html))
}
pub async fn login(
    jar: CookieJar,
    State(s): State<AppState>,
    Form(payload): Form<view_structs::admin::login::LoginForm>,
) -> AppResult<(CookieJar, Redirect)> {
    let sessions_repo = SessionRepository::new(&s);

    if let Ok(session) = sessions_repo
        .create(&payload.username.as_str(), &payload.password.as_str())
        .await
    {
        let mut cookie = Cookie::new(extract_session::SESSION_COOKIE_NAME, session.token);
        cookie.set_path("/");
        return Ok((jar.add(cookie), Redirect::to("/admin/boards")));
    }

    Err(unauthorized("Invalid credentials"))
}

pub async fn logout(
    State(s): State<AppState>,
    AdminSession(admin): AdminSession,
) -> AppResult<Html<String>> {
    let sessions_repo = SessionRepository::new(&s);
    match admin {
        Some(admin) => {
            if let Ok(_session) = sessions_repo.delete_by_token(&admin.0.token).await {
                let html = (view_structs::admin::login::LoginTemplate { validation: None })
                    .render()
                    .map_err(|_| internal_error("Template render failed"))?;
                return Ok(Html(html));
            } else {
                return Err(unauthorized("Not an admin"));
            }
        }
        None => return Err(unauthorized("Not an admin")),
    }
}

pub async fn boards(
    AdminSession(admin_session): AdminSession,
    State(s): State<AppState>,
) -> AppResult<Html<String>> {
    match admin_session {
        Some(_admin_session) => {
            let board_repo = BoardRepository::new(&s);
            let raw_boards = board_repo.list_all().await?;

            let mut boards = Vec::<Board>::new();
            for board in raw_boards {
                let board = board_repo.materialize(board).await?;
                boards.push(board);
            }

            let html = (view_structs::admin::boards::BoardsTemplate {
                boards,
                validation: None,
            })
            .render()
            .map_err(|_| internal_error("Template render failed"))?;
            Ok(Html(html))
        }
        None => return Err(unauthorized("Not an admin")),
    }
}
pub async fn display_create_board(
    State(s): State<AppState>,
    AdminSession(admin_session): AdminSession,
) -> AppResult<Html<String>> {
    let category_repo = BoardCategoryRepository::new(&s);
    let categories = category_repo.list_all().await?;

    match admin_session {
        Some(_) => {
            let html = (view_structs::admin::create_board::CreateBoardTemplate {
                categories,
                validation: None,
            })
            .render()
            .map_err(|_| internal_error("Template render failed"))?;
            Ok(Html(html))
        }
        None => return Err(unauthorized("Not an admin")),
    }
}
pub async fn create_board(
    State(s): State<AppState>,
    AdminSession(admin_session): AdminSession,
    Form(payload): Form<view_structs::admin::create_board::CreateBoardForm>,
) -> AppResult<Redirect> {
    match admin_session {
        Some((_, admin)) => {
            let category_repo = BoardCategoryRepository::new(&s);
            let board_repo = BoardRepository::new(&s);
            dbg!(&payload);
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
                                    .map_err(|_| internal_error("Invalid category UUID"))?;

                                let category = category_repo.find_by_id(parsed_uuid).await?;

                                Some(category.id)
                            }
                            None => None,
                        },
                    },
                )
                .await?;
            return Ok(Redirect::to(
                format!("/admin/boards/board/{}", board.id).as_str(),
            ));
        }
        None => return Err(unauthorized("Not an admin")),
    }
}

pub async fn view_board(
    State(s): State<AppState>,
    Path(path): Path<Uuid>,
    AdminSession(admin_session): AdminSession,
) -> AppResult<Html<String>> {
    match admin_session {
        Some(_admin_session) => {
            let board_repo = BoardRepository::new(&s);
            let category_repo = BoardCategoryRepository::new(&s);

            if let Ok(board) = board_repo.find_by_id(path).await {
                let board = board_repo.materialize(board).await?;
                let categories = category_repo.list_all().await?;
                let html = (view_structs::admin::edit_board::EditBoardTemplate {
                    validation: None,
                    board: board,
                    categories,
                })
                .render()
                .map_err(|_| internal_error("Template render failed"))?;
                return Ok(Html(html));
            }
            let html =
                (view_structs::status::error::not_found::NotFoundTemplate { board_name: None })
                    .render()
                    .map_err(|_| internal_error("Template render failed"))?;
            Ok(Html(html))
        }
        None => return Err(unauthorized("Not an admin")),
    }
}
pub async fn update_board(
    State(s): State<AppState>,
    Path(path): Path<Uuid>,
    AdminSession(admin_session): AdminSession,
    Form(payload): Form<view_structs::admin::edit_board::EditBoardForm>,
) -> AppResult<Redirect> {
    match admin_session {
        Some((_session, admin)) => {
            let board_repo = BoardRepository::new(&s);
            let category_repo = BoardCategoryRepository::new(&s);

            let board = board_repo.find_by_id(path).await?;

            let parsed_uuid = Uuid::from_str(payload.category_id.as_str())
                .map_err(|_| internal_error("Invalid category UUID"))?;

            let category = category_repo.find_by_id(parsed_uuid).await?;

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
        None => Err(unauthorized("Not an admin")),
    }
}
pub async fn delete_board(
    State(_s): State<AppState>,
    AdminSession(admin_session): AdminSession,
) -> AppResult<Html<String>> {
    match admin_session {
        Some(_) => {
            let html = (view_structs::admin::boards::BoardsTemplate {
                boards: vec![],
                validation: None,
            })
            .render()
            .map_err(|_| internal_error("Template render failed"))?;
            Ok(Html(html))
        }
        None => Err(unauthorized("Not an admin")),
    }
}

pub async fn categories(
    AdminSession(admin_session): AdminSession,
    State(s): State<AppState>,
) -> AppResult<Html<String>> {
    match admin_session {
        Some(_admin_session) => {
            let category_repo = BoardCategoryRepository::new(&s);
            let categories = category_repo.list_all().await?;
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
            .map_err(|_| internal_error("Template render failed"))?;
            Ok(Html(html))
        }
        None => Err(unauthorized("Not an admin")),
    }
}

pub async fn display_create_category(
    State(_): State<AppState>,
    AdminSession(admin_session): AdminSession,
) -> AppResult<Html<String>> {
    match admin_session {
        Some(_) => {
            let html =
                (view_structs::admin::create_category::CreateCategoryTemplate { validation: None })
                    .render()
                    .map_err(|_| internal_error("Template render failed"))?;
            Ok(Html(html))
        }
        None => Err(unauthorized("Not an admin")),
    }
}

pub async fn create_category(
    State(s): State<AppState>,
    AdminSession(admin_session): AdminSession,
    Form(payload): Form<view_structs::admin::create_category::CreateCategoryForm>,
) -> AppResult<Redirect> {
    match admin_session {
        Some((_, admin)) => {
            let category_repo = BoardCategoryRepository::new(&s);
            let category = category_repo.create(admin, payload.name).await?;
            Ok(Redirect::to(
                format!("/admin/categories/category/{}", category.id).as_str(),
            ))
        }
        None => Err(unauthorized("Not an admin")),
    }
}

pub async fn view_category(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    AdminSession(admin_session): AdminSession,
) -> AppResult<Html<String>> {
    match admin_session {
        Some(_admin_session) => {
            let category_repo = BoardCategoryRepository::new(&s);
            if let Ok(category) = category_repo.find_by_id(id).await {
                let html = (view_structs::admin::edit_category::EditCategoryTemplate {
                    validation: None,
                    category_info: Some(category),
                })
                .render()
                .map_err(|_| internal_error("Template render failed"))?;
                return Ok(Html(html));
            }
            Err(AppError {
                message: "Not found".to_string(),
                status_code: StatusCode::NOT_FOUND,
            })
        }
        None => Err(unauthorized("Not an admin")),
    }
}

pub async fn update_category(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    AdminSession(admin_session): AdminSession,
    Form(payload): Form<view_structs::admin::edit_category::EditCategoryForm>,
) -> AppResult<Redirect> {
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
                .await?;
            Ok(Redirect::to("/admin/categories"))
        }
        None => Err(unauthorized("Not an admin")),
    }
}

pub async fn delete_category(
    State(s): State<AppState>,
    Path(id): Path<Uuid>,
    AdminSession(admin_session): AdminSession,
) -> AppResult<Redirect> {
    match admin_session {
        Some((_, admin)) => {
            let category_repo = BoardCategoryRepository::new(&s);
            category_repo.delete(admin, id).await?;
            Ok(Redirect::to("/admin/categories"))
        }
        None => Err(unauthorized("Not an admin")),
    }
}

pub async fn bans(
    AdminSession(admin_session): AdminSession,
    State(s): State<AppState>,
) -> AppResult<Html<String>> {
    match admin_session {
        Some(_) => {
            let ban_repo = BanRepository::new(&s);
            let raw_bans = ban_repo.list_all().await?;

            let mut bans = Vec::with_capacity(raw_bans.len());
            for ban in raw_bans {
                let entry = ban_repo.materialize(ban).await?;
                bans.push(entry);
            }

            let html = (view_structs::admin::bans::BansTemplate { bans })
                .render()
                .map_err(|_| internal_error("Template render failed"))?;
            Ok(Html(html))
        }
        None => Err(unauthorized("Not an admin")),
    }
}

pub async fn attachment_policies(
    AdminSession(admin_session): AdminSession,
    State(s): State<AppState>,
) -> AppResult<Html<String>> {
    match admin_session {
        Some(_) => {
            let attachment_policy_repo = AttachmentPolicyRepository::new(&s);
            let policies = attachment_policy_repo.list_all().await?;

            let mut policies_materialized = Vec::<AttachmentPolicy>::new();
            for policy in policies {
                let policy = attachment_policy_repo.materialize(policy).await;
                match policy {
                    Ok(policy) => policies_materialized.push(policy),
                    Err(_) => continue,
                };
            }
            let html = (view_structs::admin::attachment_policies::AttachmentPoliciesTemplate {
                attachment_policies: policies_materialized,
            })
            .render()
            .map_err(|_| internal_error("Template render failed"))?;
            Ok(Html(html))
        }
        None => Err(unauthorized("Not an admin")),
    }
}
pub async fn show_create_attachment_policies(
    AdminSession(admin_session): AdminSession,
    State(s): State<AppState>,
) -> AppResult<Html<String>> {
    match admin_session {
        Some(_) => {
            let board_repository = BoardRepository::new(&s);
            let boards = board_repository.list_all().await?;

            let html =
                (view_structs::admin::create_attachment_policy::CreateAttachmentPolicyTemplate {
                    validation: None,
                    boards: boards,
                    supported_mime_types: SUPPORTED_MIME_TYPES
                        .iter()
                        .map(|mime| mime.to_string())
                        .collect(),
                    default: DBAttachmentPolicy::default(),
                })
                .render()
                .map_err(|_| internal_error("Template render failed"))?;
            Ok(Html(html))
        }
        None => Err(unauthorized("Not an admin")),
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
                        attachment_limit: payload.attachment_limit,
                    },
                )
                .await?;

            Ok(Redirect::to("/admin/attachment-policies"))
        }
        None => Err(unauthorized("Not an admin")),
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
                .map_err(|_| internal_error("Template render failed"))?;
            Ok(Html(html))
        }
        None => Err(unauthorized("Not an admin")),
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
                        attachment_limit: Some(payload.attachment_limit),
                    },
                )
                .await?;

            Ok(Redirect::to("/admin/attachment-policies"))
        }
        None => Err(unauthorized("Not an admin")),
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
        None => Err(unauthorized("Not an admin")),
    }
}
