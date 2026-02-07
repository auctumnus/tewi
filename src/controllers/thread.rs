use std::net::SocketAddr;

use askama::Template;
use axum::{
    Form,
    extract::{ConnectInfo, Multipart, Path, Query, State},
    http::StatusCode,
    response::{Html, Redirect},
};

use crate::{
    AppState, board_info::BoardInfo, err::{AppError, bad_request}, models::{
        boards::BoardRepository,
        posts::{AttachmentInfo, CreatePost, PostCreationTarget, PostRepository},
        threads::ThreadRepository,
    }, pagination::PaginatedRequest, parse_multipart::read_chunks_until_done, view_structs::{
        self,
        board_page::BoardPageTemplate,
        status::error::not_found::NotFoundTemplate,
    }
};

pub async fn board_page(
    BoardInfo(board, board_slugs): BoardInfo,
    Query(pagination_params): Query<PaginatedRequest>,
    State(s): State<AppState>,
) -> Result<Html<String>, StatusCode> {
    let board_repo = BoardRepository::new(&s);
    match board {
        Some(board) => {
            let threads = board_repo
                .threads_for_board(board.id, pagination_params)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let html = (BoardPageTemplate {
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

struct PostInfo {
    title: String,
    name: String,
    content: String,
    attachments: Vec<AttachmentInfo>,
}

async fn multipart_to_post_info(
    mut multipart: Multipart
) -> Result<PostInfo, AppError> {
    let mut title = String::new();
    let mut name = String::new();
    let mut content = String::new();
    let mut attachments = vec![];

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| AppError {
            message: "Failed to parse multipart form".to_owned(),
            status_code: StatusCode::BAD_REQUEST,
        })?
    {
        match field.name().unwrap_or("") {
            "title" => {
                title = field
                    .text()
                    .await
                    .map_err(|_| AppError {
                        message: "Failed to read title field".to_owned(),
                        status_code: StatusCode::BAD_REQUEST,
                    })?;
            }
            "name" => {
                name = field
                    .text()
                    .await
                    .map_err(|_| AppError {
                        message: "Failed to read name field".to_owned(),
                        status_code: StatusCode::BAD_REQUEST,
                    })?;
            }
            "content" => {
                content = field
                    .text()
                    .await
                    .map_err(|_| AppError {
                        message: "Failed to read content field".to_owned(),
                        status_code: StatusCode::BAD_REQUEST,
                    })?;
            }
            "attachments" => {
                let content_type = field
                    .content_type()
                    .ok_or(bad_request("Missing content type for attachment"))?
                    .to_string();
                let filename = field
                    .file_name()
                    .ok_or(bad_request("Missing filename for attachment"))?
                    .to_string();

                let data = read_chunks_until_done(field)
                    .await
                    .map_err(|_| AppError {
                        message: "Failed to read attachment data".to_owned(),
                        status_code: StatusCode::BAD_REQUEST,
                    })?;
                if data.is_empty() {
                    continue;
                }

                attachments.push(AttachmentInfo{
                    data,
                    content_type,
                    filename,
                });
            }
            _ => {}
        }
    }

    Ok(PostInfo {
        title,
        name,
        content,
        attachments,
    })
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
                return Err(bad_request("At least one attachment is required to create a thread"));
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
                .find_by_board_and_number(
                    board.id,
                    path.1,
                )
                .await?;

            println!("Creating post in thread {} on board {}", thread.id, board.id); // --- IGNORE ---

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

pub async fn thread(
    BoardInfo(board, board_slugs): BoardInfo,
    Path(path): Path<(String, i32)>,
    State(s): State<AppState>,
) -> Result<Html<String>, StatusCode> {
    match board {
        Some(board) => {
            let thread_repo = ThreadRepository::new(&s);

            let thread = thread_repo
                .find_by_board_and_number(
                    board.id,
                    path.1,
                )
                .await
                .map_err(|_| StatusCode::NOT_FOUND)?;

            let thread = thread_repo
                .materialize(thread)
                .await
                .map_err(|_| StatusCode::NOT_FOUND)?;

            let html = (view_structs::thread::ThreadTemplate {
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
