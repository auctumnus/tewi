use axum::body::Bytes;
use chrono::{DateTime, Utc};
use ipnetwork::IpNetwork;
use uuid::Uuid;

use crate::{
    AppState, err::{AppResult, banned, unauthorized}, markup::{MarkupRenderer, Render}, models::{
        admins::Admin,
        attachments::{Attachment, AttachmentRepository},
        bans::BanRepository,
        boards::BoardRepository,
        ips::IpRepository,
        threads::ThreadRepository,
    }
};

#[derive(sqlx::FromRow)]
pub struct Ref {
    pub id: Uuid,
    pub from_post_id: Uuid,
    pub to_post_id: Uuid,
}

#[derive(sqlx::FromRow, Debug)]
pub struct DBPost {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub ip_id: Option<Uuid>,
    pub associated_ban_id: Option<Uuid>,

    pub post_number: i32,
    pub title: String,
    pub name: String,
    pub content: String,

    pub created_at: DateTime<Utc>,
    pub hidden_at: Option<DateTime<Utc>>,
}
#[derive(Debug)]
pub struct Post {
    pub id: Uuid,
    pub board_slug: String,
    pub thread_id: Uuid,
    pub ip_id: Option<Uuid>,
    pub associated_ban_id: Option<Uuid>,
    pub attachments: Vec<Attachment>,

    pub replying_to: Vec<DBPost>,
    pub backlinks: Vec<DBPost>,

    pub post_number: i32,
    pub title: Option<String>,
    pub name: Option<String>,
    pub content: String,
    pub content_rendered: String,

    pub created_at: DateTime<Utc>,
    pub hidden_at: Option<DateTime<Utc>>,
}

pub enum AdminAction {
    Delete,
    Hide,
    Unhide,
    Sticky,
    Unsticky,
    Ban {
        reason: String,
        duration: Option<i64>,
    },
}

pub enum PostAction {
    DeleteByUser {
        ip: IpNetwork,
    },
    AdminAction {
        requestor: Admin,
        action: AdminAction,
    },
}

pub enum PostCreationTarget {
    Thread(Uuid),
    Board(Uuid),
}

pub struct AttachmentInfo {
    pub data: Bytes,
    pub content_type: String,
    pub filename: String,
}

pub struct CreatePost {
    pub target: PostCreationTarget,
    pub title: String,
    pub name: String,
    pub content: String,
    pub attachments: Vec<AttachmentInfo>,
}

fn parse_references(content: &str) -> Vec<(i32, Option<&str>)> {
    let mut references = Vec::new();
    for word in content.split_whitespace() {
        // >>>/board/123
        if let Some(stripped) = word.strip_prefix(">>>/") {
            let parts: Vec<&str> = stripped.split('/').collect();
            if parts.len() == 2 && let Ok(post_number) = parts[1].parse::<i32>() {
                references.push((post_number, Some(parts[0])));
            }
            continue;
        }

        // >>123
        if let Some(stripped) = word.strip_prefix(">>") &&
            let Ok(post_number) = stripped.parse::<i32>() {
                references.push((post_number, None));
        }
    }
    references
}

pub struct PostRepository(AppState);

impl PostRepository {
    pub fn new(state: &AppState) -> Self {
        Self(state.clone())
    }

    pub async fn create(&self, request_ip: IpNetwork, create_post: CreatePost) -> AppResult<Post> {
        let ip_repo = IpRepository::new(&self.0);
        let ip = ip_repo.find_or_create(request_ip).await?;

        let bans = BanRepository::new(&self.0);
        let ban = bans.find_active_by_ip(ip.id).await?;
        if let Some(ban) = ban {
            return Err(banned(&ban.reason));
        }

        let mut tx = self.0.db.begin().await?;

        let post_id = Uuid::new_v4();

        let board_id = match create_post.target {
            PostCreationTarget::Thread(thread_id) => {
                let board_id =
                    sqlx::query_scalar!("SELECT board_id FROM threads WHERE id = $1", thread_id)
                        .fetch_one(&mut *tx)
                        .await?;
                match board_id {
                    Some(board_id) => board_id,
                    None => return Err(unauthorized("Thread does not belong to a board")),
                }
            }
            PostCreationTarget::Board(board_id) => board_id,
        };

        let boards = BoardRepository::new(&self.0);
        let board = boards.find_by_id(board_id).await?;

        let (thread_id, post_number) = match create_post.target {
            PostCreationTarget::Thread(thread_id) => (thread_id, board.next_post_number),
            PostCreationTarget::Board(board_id) => {
                // make a new thread
                let threads = ThreadRepository::new(&self.0);
                let thread = threads.create(Some(board_id), None, &mut tx).await?;
                (thread.id, board.next_post_number)
            }
        };

        sqlx::query_scalar!(
            "INSERT INTO posts (id, thread_id, ip_id, post_number, title, name, content, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            post_id,
            thread_id,
            ip.id,
            post_number,
            create_post.title,
            create_post.name,
            create_post.content,
            Utc::now()
        )
        .execute(&mut *tx)
        .await?;

        let references = parse_references(&create_post.content);
        for &(post_number, board_slug) in &references {
            let board_id = if let Some(slug) = board_slug {
                let board = boards.find_by_slug(slug).await?;
                board.id
            } else {
                board_id
            };
            if let Ok(referenced_post) = self.find_by_board_and_number(board_id, post_number).await {
                sqlx::query!(
                    "INSERT INTO refs (from_post_id, to_post_id) VALUES ($1, $2)",
                    post_id,
                    referenced_post.id
                )
                .execute(&mut *tx)
                .await?;
            }
        }

        if matches!(create_post.target, PostCreationTarget::Board(_)) {
            let threads = ThreadRepository::new(&self.0);
            threads.add_op(
                thread_id,
                crate::models::threads::AddOpTemplate { post_id },
                &mut tx,
            ).await?;
        }


        let attachments = AttachmentRepository::new(&self.0);
        for attachment in create_post.attachments {
            let create_attachment = crate::models::attachments::CreateAttachment {
                data: attachment.data.to_vec(),
                post_id,
                mime_type: attachment.content_type,
                original_filename: attachment.filename,
                spoilered: false,
            };
            attachments.create(create_attachment, &mut tx).await?;
        }

        let _ = boards.increment_next_post_number(board_id, &mut tx).await?;

        tx.commit().await?;

        let db_post = self.find_by_id(post_id).await?;
        self.materialize(db_post).await
    }

    pub async fn materialize(&self, db_post: DBPost) -> AppResult<Post> {
        let attachment_repo = AttachmentRepository::new(&self.0);
        let attachments = attachment_repo.find_by_post_id(db_post.id).await?;

        self.materialize_from_attachments(db_post, attachments)
            .await
    }

    async fn find_replying_to(&self, post_id: Uuid, thread_id: Uuid) -> AppResult<Vec<DBPost>> {
        sqlx::query_as!(
            DBPost,
            "SELECT p.* FROM posts p
            JOIN refs r ON p.id = r.to_post_id
            JOIN threads tp ON p.thread_id = tp.id
            JOIN threads tc ON tc.id = $2
            WHERE r.from_post_id = $1 AND tp.board_id = tc.board_id AND tp.id = tc.id",
            post_id,
            thread_id
        )
        .fetch_all(&self.0.db)
        .await
        .map_err(Into::into)
    }

    async fn find_backlinks(&self, post_id: Uuid, thread_id: Uuid) -> AppResult<Vec<DBPost>> {
        sqlx::query_as!(
            DBPost,
            "SELECT p.* FROM posts p
            JOIN refs r ON p.id = r.from_post_id
            JOIN threads tp ON p.thread_id = tp.id
            JOIN threads tc ON tc.id = $2
            WHERE r.to_post_id = $1 AND tp.board_id = tc.board_id AND tp.id = tc.id",
            post_id,
            thread_id
        )
        .fetch_all(&self.0.db)
        .await
        .map_err(Into::into)
    }

    async fn get_uri(&self, post_id: Uuid) -> AppResult<String> {
        let record = sqlx::query!(
            "SELECT b.slug AS board_slug, op.post_number AS thread_op_post_number, p.post_number AS post_number
            FROM posts p
            JOIN threads t ON p.thread_id = t.id
            JOIN boards b ON t.board_id = b.id
            JOIN posts op ON t.op_post = op.id
            WHERE p.id = $1",
            post_id
        )
        .fetch_one(&self.0.db)
        .await?;

        let board_slug = record.board_slug;
        let thread_op_post_number = record.thread_op_post_number;
        let post_number = record.post_number;

        let is_op = thread_op_post_number == post_number;

        if is_op {
            return Ok(format!(
                "/board/{}/thread/{}",
                board_slug, thread_op_post_number
            ));
        }

        Ok(format!(
            "/board/{}/thread/{}#{}",
            board_slug, thread_op_post_number, post_number
        ))
    }

    pub async fn materialize_from_attachments(
        &self,
        db_post: DBPost,
        attachments: Vec<Attachment>,
    ) -> AppResult<Post> {
        let replying_to = self
            .find_replying_to(db_post.id, db_post.thread_id)
            .await?;

        let backlinks = self
            .find_backlinks(db_post.id, db_post.thread_id)
            .await?;

        let board_id = sqlx::query_scalar!(
            "SELECT board_id FROM threads WHERE id = $1",
            db_post.thread_id
        )
        .fetch_one(&self.0.db)
        .await?
        .ok_or_else(|| unauthorized("thread does not belong to a board"))?;

        let board_slug = sqlx::query_scalar!(
            "SELECT b.slug FROM boards b JOIN threads t ON b.id = t.board_id WHERE t.id = $1",
            db_post.thread_id
        )
        .fetch_one(&self.0.db)
        .await?;

        let renderer = MarkupRenderer::new(&self.0);
        let render = Render {
            content: db_post.content.clone(),
            board_id,
        };
        let content_rendered = renderer.render(render).await?;

        Ok(Post {
            id: db_post.id,
            thread_id: db_post.thread_id,
            ip_id: db_post.ip_id,
            associated_ban_id: db_post.associated_ban_id,
            attachments,
            post_number: db_post.post_number,
            title: match db_post.title.is_empty() {
                true => None,
                false => Some(db_post.title),
            },
            name: match db_post.name.is_empty() {
                true => None,
                false => Some(db_post.name),
            },
            content: db_post.content,
            content_rendered,
            created_at: db_post.created_at,
            hidden_at: db_post.hidden_at,
            replying_to,
            backlinks,
            board_slug,
        })
    }

    pub async fn find_by_id(&self, post_id: Uuid) -> AppResult<DBPost> {
        sqlx::query_as::<_, DBPost>("SELECT * FROM posts WHERE id = $1")
            .bind(post_id)
            .fetch_one(&self.0.db)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_board_and_number(
        &self,
        board_id: Uuid,
        post_number: i32,
    ) -> AppResult<DBPost> {
        sqlx::query_as::<_, DBPost>("SELECT p.* FROM posts p JOIN threads t ON p.thread_id = t.id WHERE t.board_id = $1 AND p.post_number = $2")
            .bind(board_id)
            .bind(post_number)
            .fetch_one(&self.0.db)
            .await
            .map_err(Into::into)
    }

    pub async fn perform_action(&self, action: PostAction, post_id: Uuid) -> AppResult<()> {
        let db_post = self.find_by_id(post_id).await?;
        match action {
            PostAction::DeleteByUser { ip } => {
                let Some(ip_id) = db_post.ip_id else {
                    // If the post doesn't have an associated IP, we can't verify the user's IP
                    return Err(unauthorized("no ip associated with post"));
                };

                // Check if the IP address matches the post's IP
                let ip_repo = IpRepository::new(&self.0);
                let post_ip = ip_repo.find_by_id(ip_id).await?;
                if post_ip.ip_address != ip {
                    return Err(unauthorized("IP address does not match post's IP"));
                }

                // If the IP address matches, proceed with deleting the post
                sqlx::query("DELETE FROM posts WHERE id = $1")
                    .bind(post_id)
                    .execute(&self.0.db)
                    .await?;
            }
            PostAction::AdminAction { requestor, action } => {
                match action {
                    AdminAction::Delete => {
                        tracing::info!("Admin {} is deleting post {}", requestor.name, post_id);
                        sqlx::query("DELETE FROM posts WHERE id = $1")
                            .bind(post_id)
                            .execute(&self.0.db)
                            .await?;
                    }
                    AdminAction::Hide => {
                        tracing::info!("Admin {} is hiding post {}", requestor.name, post_id);
                        sqlx::query("UPDATE posts SET hidden_at = NOW() WHERE id = $1")
                            .bind(post_id)
                            .execute(&self.0.db)
                            .await?;
                    }
                    AdminAction::Unhide => {
                        tracing::info!("Admin {} is unhiding post {}", requestor.name, post_id);
                        sqlx::query("UPDATE posts SET hidden_at = NULL WHERE id = $1")
                            .bind(post_id)
                            .execute(&self.0.db)
                            .await?;
                    }
                    AdminAction::Sticky => {
                        tracing::info!("Admin {} is sticking post {}", requestor.name, post_id);
                        sqlx::query("UPDATE posts SET sticky = TRUE WHERE id = $1")
                            .bind(post_id)
                            .execute(&self.0.db)
                            .await?;
                    }
                    AdminAction::Unsticky => {
                        tracing::info!("Admin {} is unsticking post {}", requestor.name, post_id);
                        sqlx::query("UPDATE posts SET sticky = FALSE WHERE id = $1")
                            .bind(post_id)
                            .execute(&self.0.db)
                            .await?;
                    }
                    AdminAction::Ban { reason, duration } => {
                        let Some(ip_id) = db_post.ip_id else {
                            // If the post doesn't have an associated IP, we can't verify the user's IP
                            return Err(unauthorized("no ip associated with post"));
                        };

                        let ip_repo = IpRepository::new(&self.0);
                        let post_ip = ip_repo.find_by_id(ip_id).await?;

                        tracing::info!(
                            "Admin {} is banning IP {} for reason '{}' and duration {:?}",
                            requestor.name,
                            post_ip.ip_address,
                            reason,
                            duration
                        );
                        sqlx::query(
                            "INSERT INTO bans (ip_id, reason, duration) VALUES ($1, $2, $3)",
                        )
                        .bind(ip_id)
                        .bind(reason)
                        .bind(duration)
                        .execute(&self.0.db)
                        .await?;
                    }
                }
            }
        }
        Ok(())
    }
}
