use std::collections::HashMap;

use askama::Template;
use axum::{
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::{Html, Redirect},
};

use crate::{
    AppState,
    board_info::BoardInfo,
    models::{
        attachments::{Attachment, AttachmentRepository},
        boards::BoardRepository,
        posts::PostRepository,
        threads::ThreadRepository,
    },
    pagination::PaginatedRequest,
    view_structs::{
        self, board_page::BoardPageTemplate, status::error::not_found::NotFoundTemplate,
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

pub async fn create_thread(
    Path(path): Path<String>,
    //Query(query): Query<PaginatedRequest>,
    State(s): State<AppState>,
    //ClientIp(ip): ClientIp,
    mut multipart: Multipart,
) -> Result<Redirect, StatusCode> {
    let board_repo = BoardRepository::new(&s);
    let post_repo = PostRepository::new(&s);
    let attachment_repo = AttachmentRepository::new(&s);
    let board = board_repo.find_by_slug(&path).await;

    match board {
        Ok(board) => {
            let mut attachments = Vec::<Attachment>::new();
            let mut text_fields = HashMap::<String, String>::new();

            println!("DESU 2");
            while let Some(field) = multipart
                .next_field()
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            {
                println!("DESU 3");
                //dbg!(&field);
                let name = field
                    .name()
                    .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
                    .to_string();
                println!("DESU 4, {}", name);
                println!("DESU 4, {:#?}", &field.headers());

                match &field.content_type() {
                    Some(content_type) => {
                        println!("DESU 5, {:#?}", content_type);
                        println!("DESU 6");
                        /* let data = field.bytes().await.map_err(|_| {
                            println!("desu desu desu");
                            StatusCode::INTERNAL_SERVER_ERROR
                        })?; */
                        println!("DESU 7");
                        /*  let attachment = attachment_repo
                            .create(CreateAttachment {
                                data: data.into(),
                                post_id: Uuid::new_v4(),
                                mime_type,
                                original_filename: name,
                                spoilered: false,
                            })
                            .await
                            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                        dbg!(&attachment.id);
                        attachments.push(attachment); */
                    }
                    None => {
                        println!("Desu 5, Nope");
                        let data = field
                            .text()
                            .await
                            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

                        println!("Desu 6, {}", data);

                        text_fields.insert(
                            (match name.as_str() {
                                "name" => Ok("name"),
                                "title" => Ok("title"),
                                "content" => Ok("content"),
                                _ => Err(StatusCode::BAD_REQUEST),
                            })?
                            .to_string(),
                            data,
                        );
                    }
                };
                println!("DESU 8");
            }

            println!("DESU 9");
            println!("AWAWAWAWAWAWAWAWA, {:#?}", text_fields);

            return Ok(Redirect::to(format!("/boards/{}", board.slug).as_str()));

            /* let thread = post_repo
                .create(
                    ip.into(),
                    CreatePost {
                        target: PostCreationTarget::Board(board.id),
                        title: payload.title,
                        name: payload.name,
                        content: payload.content,
                        attachments,
                    },
                )
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            return Ok(Redirect::to(
                format!("/boards/{}/thread/{}", board.slug, thread.post_number,).as_str(),
            )); */
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
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
    State(s): State<AppState>,
) -> Result<Html<String>, StatusCode> {
    return match board {
        Some(board) => {
            let board_repo = BoardRepository::new(&s);
            let thread_repo = ThreadRepository::new(&s);

            let boards = board_repo
                .list_all()
                .await
                .map_err(|_| StatusCode::NOT_FOUND)?;

            let thread = thread_repo
                .find_by_board_and_number(board.id, 1)
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
