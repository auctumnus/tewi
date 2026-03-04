use std::net::SocketAddr;

use askama::Template;
use axum::{
    Form,
    extract::{ConnectInfo, Multipart, Path, Query, State},
    http::StatusCode,
    response::{Html, Redirect},
};
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    board_info::BoardInfo,
    controllers::common::form::{PostInfo, multipart_to_post_info},
    err::{AppError, AppResult, bad_request},
    extract_session::AdminSession,
    models::{
        boards::BoardRepository,
        posts::{CreatePost, PostCreationTarget, PostRepository},
        threads::{self, ThreadAction, ThreadRepository},
    },
    pagination::PaginatedRequest,
    view_structs::{
        self, board_page::BoardPageTemplate, status::error::not_found::NotFoundTemplate,
    },
};

#[derive(Deserialize, Debug, Clone, Serialize)]
enum ThreadDeleteOptions {
    Sticky(bool),
    Close(bool),
    Hide(bool),
}

impl TryFrom<view_structs::thread_admin_form::ThreadAdminForm> for ThreadDeleteOptions {
    type Error = AppError;
    fn try_from(value: view_structs::thread_admin_form::ThreadAdminForm) -> AppResult<Self> {
        match value.action.as_str() {
            "sticky" => Ok(ThreadDeleteOptions::Sticky(true)),
            "unsticky" => Ok(ThreadDeleteOptions::Sticky(false)),
            "close" => Ok(ThreadDeleteOptions::Close(true)),
            "unclose" => Ok(ThreadDeleteOptions::Close(false)),
            "hide" => Ok(ThreadDeleteOptions::Hide(true)),
            "unhide" => Ok(ThreadDeleteOptions::Hide(false)),
            _ => Err(bad_request("Invalid delete action")),
        }
    }
}

pub async fn board_page(
    BoardInfo(board, board_slugs): BoardInfo,
    Query(pagination_params): Query<PaginatedRequest>,
    State(s): State<AppState>,
    AdminSession(admin_session): AdminSession,
) -> Result<Html<String>, StatusCode> {
    let board_repo = BoardRepository::new(&s);
    match board {
        Some(board) => {
            let threads = board_repo
                .threads_for_board(board.id, pagination_params, admin_session.is_none())
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let html = (BoardPageTemplate {
                admin_session,
                form_route: format!("/board/{}", board.slug.clone()),
                board,
                board_slugs,
                threads,
            })
            .render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Html(html))
        }
        None => {
            let html = (NotFoundTemplate { board_name: None })
                .render()
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Html(html))
        }
    }
}

pub async fn create_thread(
    BoardInfo(board, _): BoardInfo,
    State(s): State<AppState>,
    ConnectInfo(connection_info): ConnectInfo<SocketAddr>,
    multipart: Multipart,
) -> Result<Redirect, AppError> {
    let post_repo = PostRepository::new(&s);

    match board {
        Some(board) => {
            let post_info = multipart_to_post_info(multipart).await?;
            let PostInfo {
                title,
                name,
                content,
                attachments,
            } = post_info;

            if attachments.is_empty() {
                return Err(bad_request(
                    "At least one attachment is required to create a thread",
                ));
            }

            let attachment_limit = board.attachment_policy.attachment_limit;

            if attachments.len() as i64 > attachment_limit {
                return Err(bad_request(
                    format!("Too many attachments, limit is {}", attachment_limit).as_str(),
                ));
            }

            let op_post = post_repo
                .create(
                    connection_info.ip().into(),
                    CreatePost {
                        target: PostCreationTarget::Board(board.id),
                        title,
                        name,
                        content,
                        attachments,
                    },
                )
                .await?;

            Ok(Redirect::to(
                format!("/board/{}/thread/{}", board.slug, op_post.post_number).as_str(),
            ))
        }
        None => Err(AppError {
            message: "Board not found".to_owned(),
            status_code: StatusCode::NOT_FOUND,
        }),
    }
}

pub async fn thread(
    BoardInfo(board, board_slugs): BoardInfo,
    Path(path): Path<(String, i32)>,
    AdminSession(admin_session): AdminSession,
    State(s): State<AppState>,
) -> Result<Html<String>, StatusCode> {
    match board {
        Some(board) => {
            let thread_repo = ThreadRepository::new(&s);

            let thread = thread_repo
                .find_by_board_and_number(board.id, path.1)
                .await
                .map_err(|_| StatusCode::NOT_FOUND)?;

            let thread = thread_repo
                .materialize(thread, None)
                .await
                .map_err(|_| StatusCode::NOT_FOUND)?;

            let html = (view_structs::thread::ThreadTemplate {
                admin_session,
                form_route: format!("/board/{}/thread/{}", board.slug.clone(), path.1),
                board,
                board_slugs,
                thread,
            })
            .render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Html(html))
        }
        None => Err(StatusCode::NOT_FOUND),
    }
}

pub async fn delete_thread(
    BoardInfo(board, _): BoardInfo,
    State(s): State<AppState>,
    Path(path): Path<(String, i32)>,
    AdminSession(admin_session): AdminSession,
    Form(payload): Form<view_structs::thread_admin_form::ThreadAdminForm>,
) -> AppResult<Redirect> {
    match admin_session {
        Some((_, admin)) => {
            let board = board.ok_or(AppError {
                message: "Not a post".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;

            let threads_repo = ThreadRepository::new(&s);

            let thread = threads_repo
                .find_by_board_and_number(board.id, path.1)
                .await
                .map_err(|_| AppError {
                    message: "Not a thread".to_string(),
                    status_code: StatusCode::NOT_FOUND,
                })?;

            let thread = threads_repo
                .materialize(thread, None)
                .await
                .map_err(|_| AppError {
                    message: "Not a thread".to_string(),
                    status_code: StatusCode::NOT_FOUND,
                })?;

            let delete_params = ThreadDeleteOptions::try_from(payload)?;
            match delete_params {
                ThreadDeleteOptions::Sticky(sticky) => {
                    threads_repo
                        .perform_action(
                            ThreadAction::AdminAction {
                                requestor: admin,
                                action: threads::AdminAction::Sticky(sticky),
                            },
                            thread.id,
                        )
                        .await?;
                }
                ThreadDeleteOptions::Hide(hide) => {
                    threads_repo
                        .perform_action(
                            ThreadAction::AdminAction {
                                requestor: admin,
                                action: threads::AdminAction::Hide(hide),
                            },
                            thread.id,
                        )
                        .await?;
                }
                ThreadDeleteOptions::Close(close) => {
                    let _ = threads_repo
                        .perform_action(
                            ThreadAction::AdminAction {
                                requestor: admin,
                                action: threads::AdminAction::Close(close),
                            },
                            thread.id,
                        )
                        .await
                        .map_err(|_| AppError {
                            message: "Action failed".to_string(),
                            status_code: StatusCode::INTERNAL_SERVER_ERROR,
                        })?;
                }
            }

            return Ok(Redirect::to(
                format!("/board/{}/thread/{}", board.slug, path.1).as_str(),
            ));
        }
        None => Err(AppError {
            message: "Not an Admin".to_string(),
            status_code: StatusCode::UNAUTHORIZED,
        }),
    }
}
