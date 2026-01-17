
use std::net::IpAddr;

use chrono::{DateTime, Utc};
use ipnetwork::IpNetwork;
use uuid::Uuid;

use crate::{AppState, err::{AppResult, banned, unauthorized}, models::{admins::Admin, attachments::{Attachment, AttachmentRepository}, bans::BanRepository, boards::BoardRepository, ips::IpRepository, threads::ThreadRepository}};

#[derive(sqlx::FromRow)]
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

pub struct Post {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub ip_id: Option<Uuid>,
    pub associated_ban_id: Option<Uuid>,
    pub attachments: Vec<Attachment>,

    pub post_number: i32,
    pub title: String,
    pub name: String,
    pub content: String,

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
    }
}

pub enum PostAction {
    DeleteByUser {
        ip: IpNetwork,
    },
    AdminAction {
        requestor: Admin,
        action: AdminAction,
    }
}

pub enum PostCreationTarget {
    Thread(Uuid),
    Board(Uuid),
}

pub struct CreatePost {
    pub target: PostCreationTarget,
    pub title: String,
    pub name: String,
    pub content: String,
    pub attachments: Vec<Attachment>,
}

pub struct PostRepository(AppState);

impl PostRepository {
    pub fn new(state: &AppState) -> Self {
        Self(state.clone())
    }

    // TODO: do this in a transaction
    pub async fn create(&self, request_ip: IpNetwork, create_post: CreatePost) -> AppResult<Post> {
        let ip_repo = IpRepository::new(&self.0);
        let ip = ip_repo.find_or_create(request_ip).await?;

        let bans = BanRepository::new(&self.0);
        let ban = bans.find_active_by_ip(ip.id).await?;
        if let Some(ban) = ban {
            return Err(banned(&ban.reason));
        }

        let post_id = Uuid::new_v4();
        let board_id = match create_post.target {
            PostCreationTarget::Thread(thread_id) => {
                let board_id = sqlx::query_scalar!(
                    "SELECT board_id FROM threads WHERE id = $1",
                    thread_id
                )
                .fetch_one(&self.0.db)
                .await?;
                match board_id {
                    Some(board_id) => board_id,
                    None => return Err(unauthorized("Thread does not belong to a board")),
                }
            },
            PostCreationTarget::Board(board_id) => board_id,
        };
        let boards = BoardRepository::new(&self.0);
        let board = boards.find_by_id(board_id).await?;
        let (thread_id, post_number) = match create_post.target {
            PostCreationTarget::Thread(thread_id) => {
                (thread_id, board.next_post_number)
            },
            PostCreationTarget::Board(board_id) => {
                // make a new thread
                let threads = ThreadRepository::new(&self.0);
                let thread = threads.create(Some(board_id), None).await?;
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
        .execute(&self.0.db)
        .await?;

        let _ = boards.increment_next_post_number(board_id).await?;

        let db_post = self.find_by_id(post_id).await?;
        self.materialize(db_post).await
    }

    pub async fn materialize(&self, db_post: DBPost) -> AppResult<Post> {
        let attachment_repo = AttachmentRepository::new(&self.0);
        let attachments = attachment_repo.find_by_post_id(db_post.id).await?;

        self.materialize_from_attachments(db_post, attachments).await
    }

    pub async fn materialize_from_attachments(&self, db_post: DBPost, attachments: Vec<Attachment>) -> AppResult<Post> {
        Ok(Post {
            id: db_post.id,
            thread_id: db_post.thread_id,
            ip_id: db_post.ip_id,
            associated_ban_id: db_post.associated_ban_id,
            attachments,
            post_number: db_post.post_number,
            title: db_post.title,
            name: db_post.name,
            content: db_post.content,
            created_at: db_post.created_at,
            hidden_at: db_post.hidden_at,
        })
    }

    pub async fn find_by_id(&self, post_id: Uuid) -> AppResult<DBPost> {
        sqlx::query_as::<_, DBPost>(
            "SELECT * FROM posts WHERE id = $1"
        )
        .bind(post_id)
        .fetch_one(&self.0.db)
        .await
        .map_err(Into::into)
    }

    pub async fn find_by_board_and_number(&self, board_id: Uuid, post_number: i32) -> AppResult<DBPost> {
        sqlx::query_as::<_, DBPost>(
            "SELECT * FROM posts WHERE board_id = $1 AND post_number = $2"
        )
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
                    },
                    AdminAction::Unhide => {
                        tracing::info!("Admin {} is unhiding post {}", requestor.name, post_id);
                        sqlx::query("UPDATE posts SET hidden_at = NULL WHERE id = $1")
                            .bind(post_id)
                            .execute(&self.0.db)
                            .await?;
                    },
                    AdminAction::Sticky => {
                        tracing::info!("Admin {} is sticking post {}", requestor.name, post_id);
                        sqlx::query("UPDATE posts SET sticky = TRUE WHERE id = $1")
                            .bind(post_id)
                            .execute(&self.0.db)
                            .await?;
                    },
                    AdminAction::Unsticky => {
                        tracing::info!("Admin {} is unsticking post {}", requestor.name, post_id);
                        sqlx::query("UPDATE posts SET sticky = FALSE WHERE id = $1")
                            .bind(post_id)
                            .execute(&self.0.db)
                            .await?;
                    },
                    AdminAction::Ban { reason, duration } => {
                        let Some(ip_id) = db_post.ip_id else {
                            // If the post doesn't have an associated IP, we can't verify the user's IP
                            return Err(unauthorized("no ip associated with post"));
                        };

                        let ip_repo = IpRepository::new(&self.0);
                        let post_ip = ip_repo.find_by_id(ip_id).await?;

                        tracing::info!("Admin {} is banning IP {} for reason '{}' and duration {:?}", requestor.name, post_ip.ip_address, reason, duration);
                        sqlx::query("INSERT INTO bans (ip_id, reason, duration) VALUES ($1, $2, $3)")
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