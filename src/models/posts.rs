
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{AppState, err::AppResult, models::attachments::{Attachment, AttachmentRepository}};

#[derive(sqlx::FromRow)]
pub struct DBPost {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub ip_id: Option<Uuid>,
    pub associated_ban_id: Option<Uuid>,
    pub attachment_id: Option<Uuid>,

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
    pub attachment_id: Option<Uuid>,

    pub attachment: Option<Attachment>,

    pub post_number: i32,
    pub title: String,
    pub name: String,
    pub content: String,

    pub created_at: DateTime<Utc>,
    pub hidden_at: Option<DateTime<Utc>>,
}


pub struct PostRepository(AppState);

impl PostRepository {
    pub fn new(state: &AppState) -> Self {
        Self(state.clone())
    }

    pub async fn materialize(&self, db_post: DBPost) -> AppResult<Post> {
        let attachment = match db_post.attachment_id {
            Some(attachment_id) => {
                let attachment_repo = AttachmentRepository::new(&self.0);
                Some(attachment_repo.find_by_id(attachment_id).await?)
            }
            None => None,
        };

        Ok(Post {
            id: db_post.id,
            thread_id: db_post.thread_id,
            ip_id: db_post.ip_id,
            associated_ban_id: db_post.associated_ban_id,
            attachment_id: db_post.attachment_id,
            attachment,
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
}