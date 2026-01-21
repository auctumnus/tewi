use std::{collections::HashMap, net::SocketAddr};

use askama::Template;
use axum::{
    extract::{ConnectInfo, Multipart, Path, Query, State},
    http::StatusCode,
    response::{Html, Redirect},
};
use serde::Deserialize;

use crate::{
    AppState,
    board_info::BoardInfo,
    models::{
        attachments::{Attachment, AttachmentRepository},
        boards::BoardRepository,
        posts::{CreatePost, PostCreationTarget, PostRepository},
        threads::ThreadRepository,
    },
    pagination::PaginatedRequest,
    parse_multipart,
    view_structs::{
        self,
        board_page::{BoardPageTemplate, PostForm},
        status::error::not_found::NotFoundTemplate,
    },
};

pub async fn board_page(
    Path(path): Path<String>,
    BoardInfo(board, board_slugs): BoardInfo,
    Query(query): Query<PaginatedRequest>,
    State(s): State<AppState>,
) -> Result<Html<String>, StatusCode> {
    let board_repo = BoardRepository::new(&s);
    match board {
        Some(board) => {
            let threads = board_repo
                .threads_for_board(
                    board.id,
                    PaginatedRequest {
                        limit: 1000,
                        offset: 0,
                    },
                )
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            dbg!(&threads);
            let html = (BoardPageTemplate {
                board_name: Some(board.name),
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

enum FormFieldErrors {
    Missing(String),
}

fn hashmap_to_post_form_text_fields(
    data: HashMap<String, String>,
) -> Result<PostForm, FormFieldErrors> {
    Ok(PostForm {
        name: data
            .get("name")
            .ok_or(FormFieldErrors::Missing("name".to_owned()))?
            .clone(),
        title: data
            .get("title")
            .ok_or(FormFieldErrors::Missing("name".to_owned()))?
            .clone(),
        attachments: data
            .get("attachments")
            .ok_or(FormFieldErrors::Missing("name".to_owned()))?
            .clone(),
        content: data
            .get("content")
            .ok_or(FormFieldErrors::Missing("name".to_owned()))?
            .clone(),
    })
}

pub async fn create_thread(
    BoardInfo(board, _): BoardInfo,
    State(s): State<AppState>,
    ConnectInfo(connection_info): ConnectInfo<SocketAddr>,
    multipart: Multipart,
) -> Result<Redirect, StatusCode> {
    let post_repo = PostRepository::new(&s);
    let thread_repo = ThreadRepository::new(&s);

    match board {
        Some(board) => {
            let attachments = Vec::<Attachment>::new();

            let mut parsed = parse_multipart::parse_multipart::<PostForm, PostForm>(multipart)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            parsed
                .fields
                .insert("attachments".to_string(), "attachments".to_string());

            let form_fields = hashmap_to_post_form_text_fields(parsed.fields)
                .map_err(|_| StatusCode::BAD_REQUEST)?;

            let op_post = post_repo
                .create(
                    connection_info.ip().into(),
                    CreatePost {
                        target: PostCreationTarget::Board(board.id),
                        title: form_fields.title,
                        name: form_fields.name,
                        content: form_fields.content,
                        attachments,
                    },
                )
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let thread = thread_repo
                .find_by_id(op_post.thread_id)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            dbg!(&op_post);

            return Ok(Redirect::to(
                format!("/board/{}/thread/{}", board.slug, op_post.post_number).as_str(),
            ));
        }
        None => Err(StatusCode::NOT_FOUND),
    }
} /* 
pub async fn create_post(
Path(path): Path<(String, String)>,
//Query(query): Query<PaginatedRequest>,
State(s): State<AppState>,
//ClientIp(ip): ClientIp,
mut multipart: Multipart,
) -> Result<Redirect, StatusCode> {
/*  let board_repo = BoardRepository::new(&s);
    let post_repo = PostRepository::new(&s);
    let attachment_repo = AttachmentRepository::new(&s);
    let thread_repo = ThreadRepository::new(&s);

    let board = board_repo
        .find_by_slug(&path.0)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let thread = thread_repo
        .find_by_board_and_number(board.id, 1)
        .await
        .map_err(|_| StatusCode::NOT_FOUND)?;

    let op_post = thread.op_post.ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut attachments = Vec::<Attachment>::new();

    let post = post_repo
        .create(
            ip.into(),
            CreatePost {
                target: PostCreationTarget::Thread(thread.id),
                title: "".to_string(),
                name: "".to_string(),
                content: "payload.content".to_string(),
                attachments,
            },
        )
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    return Ok(Redirect::to(
        format!(
            "/boards/{}/thread/{}#{}",
            board.slug, thread.op_post.post_number, post.post_number
        )
        .as_str(),
    )); */
}
 */

pub async fn thread(
    BoardInfo(board, board_slugs): BoardInfo,
    Path(path): Path<(String, String)>,
    State(s): State<AppState>,
) -> Result<Html<String>, StatusCode> {
    return match board {
        Some(board) => {
            let thread_repo = ThreadRepository::new(&s);

            let thread = thread_repo
                .find_by_board_and_number(
                    board.id,
                    path.1.parse::<i32>().map_err(|_| StatusCode::NOT_FOUND)?,
                )
                .await
                .map_err(|_| StatusCode::NOT_FOUND)?;

            let thread = thread_repo
                .materialize(thread)
                .await
                .map_err(|_| StatusCode::NOT_FOUND)?;

            let html = (view_structs::thread::ThreadTemplate {
                board_name: Some(board.name),
                board_slugs,
                thread,
            })
            .render()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            Ok(Html(html))
        }
        None => Err(StatusCode::NOT_FOUND),
    };
}
