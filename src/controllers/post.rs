use std::net::SocketAddr;

use axum::{
    Form,
    extract::{ConnectInfo, Multipart, Path, State},
    http::StatusCode,
    response::Redirect,
};
use serde::{Deserialize, Serialize};

use crate::{
    AppState,
    board_info::BoardInfo,
    controllers::common::form::{PostInfo, multipart_to_post_info},
    err::{AppError, AppResult, bad_request, malformed},
    extract_session::AdminSession,
    models::{
        posts::{AdminAction, CreatePost, PostAction, PostCreationTarget, PostRepository},
        threads::ThreadRepository,
    },
    view_structs::{self},
};

pub enum BanOptions {
    Ban { reason: String, duration: i64 },
    BanDelete { reason: String, duration: i64 },
}

fn datetime_timezone_to_unix(datetime: String, timezone: String) -> AppResult<i64> {
    let timezone_int = timezone
        .parse::<i64>()
        .map_err(|_| malformed("Can't parse timezone"))?;

    let tz_hour = (timezone_int / 60) * 100;
    let tz_min = timezone_int % 60;
    let tz_value = tz_hour + tz_min;

    let formatted_tz = format!("{:0>4}", tz_value.abs());
    let tz_sign = match tz_value.ge(&(0 as i64)) {
        true => "+",
        false => "-",
    };

    let date_string = format!("{} {}{}", datetime.clone(), tz_sign, formatted_tz);

    let chrono_dt = chrono::DateTime::parse_from_str(date_string.as_str(), "%Y-%m-%dT%H:%M %z")
        .map_err(|err| malformed(err.to_string().as_str()))?;

    Ok(chrono_dt.timestamp())
}

impl TryFrom<view_structs::ban_form::BanForm> for BanOptions {
    type Error = AppError;
    fn try_from(value: view_structs::ban_form::BanForm) -> AppResult<Self> {
        match value.also_delete {
            None => Ok(BanOptions::Ban {
                reason: value.reason,
                duration: datetime_timezone_to_unix(value.duration, value.timezone)
                    .map(|timestamp| timestamp)?,
            }),
            Some(_) => Ok(BanOptions::BanDelete {
                reason: value.reason,
                duration: datetime_timezone_to_unix(value.duration, value.timezone)
                    .map(|timestamp| timestamp)?,
            }),
        }
    }
}

#[derive(Deserialize, Debug, Clone, Serialize)]
pub enum PostDeleteOptions {
    Remove { content: bool, attachment: bool },
    Hide,
}

impl TryFrom<view_structs::delete_form::DeleteForm> for PostDeleteOptions {
    type Error = AppError;
    fn try_from(value: view_structs::delete_form::DeleteForm) -> AppResult<Self> {
        match value.action.as_str() {
            "remove" => Ok(PostDeleteOptions::Remove {
                content: value.remove_content.unwrap_or(false),
                attachment: value.remove_attachment.unwrap_or(false),
            }),
            "hide" => Ok(PostDeleteOptions::Hide),
            _ => Err(bad_request("Invalid delete action")),
        }
    }
}

pub async fn create_post(
    BoardInfo(board, _): BoardInfo,
    Path(path): Path<(String, i32)>,
    State(s): State<AppState>,
    ConnectInfo(connection_info): ConnectInfo<SocketAddr>,
    multipart: Multipart,
) -> Result<Redirect, AppError> {
    let post_repo = PostRepository::new(&s);
    let thread_repo = ThreadRepository::new(&s);

    match board {
        Some(board) => {
            let post_info = multipart_to_post_info(multipart).await?;
            let PostInfo {
                title,
                name,
                content,
                attachments,
            } = post_info;

            let thread = thread_repo
                .find_by_board_and_number(board.id, path.1)
                .await?;

            if thread.closed_at.is_some() {
                return Err(AppError {
                    message: "Thread closed".to_string(),
                    status_code: StatusCode::FORBIDDEN,
                });
            }

            let attachment_limit = board.attachment_policy.attachment_limit;
            dbg!(&attachment_limit);

            if attachments.len() as i64 > attachment_limit {
                return Err(bad_request(
                    format!("Too many attachments, limit is {}", attachment_limit).as_str(),
                ));
            }

            println!(
                "Creating post in thread {} on board {}",
                thread.id, board.id
            ); // --- IGNORE ---

            post_repo
                .create(
                    connection_info.ip().into(),
                    CreatePost {
                        target: PostCreationTarget::Thread(thread.id),
                        title,
                        name,
                        content,
                        attachments,
                    },
                )
                .await?;

            println!("Post created in thread {}", thread.id); // --- IGNORE ---

            Ok(Redirect::to(
                format!("/board/{}/thread/{}", board.slug, path.1).as_str(),
            ))
        }
        None => Err(AppError {
            message: "Board not found".to_owned(),
            status_code: StatusCode::NOT_FOUND,
        }),
    }
}

pub async fn edit_post(
    BoardInfo(board, _): BoardInfo,
    State(s): State<AppState>,
    Path(path): Path<(String, i32, i32)>,
    AdminSession(admin_session): AdminSession,
    Form(payload): Form<view_structs::ban_form::BanForm>,
) -> AppResult<Redirect> {
    match admin_session {
        Some((_, admin)) => {
            let board = board.ok_or(AppError {
                message: "Not a post".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;

            let posts_repo = PostRepository::new(&s);

            let post = posts_repo
                .find_by_board_and_number(board.id, path.2)
                .await
                .map_err(|_| AppError {
                    message: "Not a post".to_string(),
                    status_code: StatusCode::NOT_FOUND,
                })?;

            let desu = BanOptions::try_from(payload)?;
            match desu {
                BanOptions::Ban { reason, duration } => {
                    let _ = posts_repo
                        .perform_action(
                            PostAction::AdminAction {
                                requestor: admin,
                                action: AdminAction::Ban { reason, duration },
                            },
                            post.id,
                        )
                        .await
                        .map_err(|_| AppError {
                            message: "Action failed".to_string(),
                            status_code: StatusCode::INTERNAL_SERVER_ERROR,
                        })?;
                    return Ok(Redirect::to(
                        format!("/board/{}/thread/{}#{}", board.slug, path.1, path.2).as_str(),
                    ));
                }
                BanOptions::BanDelete { reason, duration } => {
                    let _ = posts_repo
                        .perform_action(
                            PostAction::AdminAction {
                                requestor: admin,
                                action: AdminAction::Ban { reason, duration },
                            },
                            post.id,
                        )
                        .await
                        .map_err(|_| AppError {
                            message: "Action failed".to_string(),
                            status_code: StatusCode::INTERNAL_SERVER_ERROR,
                        })?;

                    /*  let _ = posts_repo
                    .perform_action(
                        PostAction::AdminAction {
                            requestor: admin,
                            action: AdminAction::Delete,
                        },
                        post.id,
                    )
                    .await
                    .map_err(|_| AppError {
                        message: "Delete failed".to_string(),
                        status_code: StatusCode::INTERNAL_SERVER_ERROR,
                    })?; */
                    return Ok(Redirect::to(
                        format!("/board/{}/thread/{}#{}", board.slug, path.1, path.2).as_str(),
                    ));
                }
            }
        }
        None => Err(AppError {
            message: "Not an Admin".to_string(),
            status_code: StatusCode::UNAUTHORIZED,
        }),
    }
}

pub async fn delete_post(
    BoardInfo(board, _): BoardInfo,
    State(s): State<AppState>,
    Path(path): Path<(String, i32, i32)>,
    AdminSession(admin_session): AdminSession,
    Form(payload): Form<view_structs::delete_form::DeleteForm>,
) -> AppResult<Redirect> {
    match admin_session {
        Some((_, admin)) => {
            let board = board.ok_or(AppError {
                message: "Not a post".to_string(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            })?;

            let posts_repo = PostRepository::new(&s);

            let post = posts_repo
                .find_by_board_and_number(board.id, path.2)
                .await
                .map_err(|_| AppError {
                    message: "Not a post".to_string(),
                    status_code: StatusCode::NOT_FOUND,
                })?;

            let delete_options = PostDeleteOptions::try_from(payload)?;
            match delete_options {
                PostDeleteOptions::Remove {
                    content,
                    attachment,
                } => {
                    let _ = posts_repo
                        .perform_action(
                            PostAction::AdminAction {
                                requestor: admin,
                                action: AdminAction::Remove {
                                    content,
                                    attachment,
                                },
                            },
                            post.id,
                        )
                        .await?;
                }
                PostDeleteOptions::Hide => {}
            }

            return Ok(Redirect::to(
                format!("/board/{}/thread/{}#{}", board.slug, path.1, path.2).as_str(),
            ));
        }
        None => Err(AppError {
            message: "Not an Admin".to_string(),
            status_code: StatusCode::UNAUTHORIZED,
        }),
    }
}
