/*
create table posts (
    id UUID PRIMARY KEY DEFAULT uuidv7(),
    thread_id UUID REFERENCES threads(id) ON DELETE CASCADE,
    ip_id UUID REFERENCES ips(id) ON DELETE SET NULL,
    associated_ban_id UUID REFERENCES bans(id) ON DELETE SET NULL,
    attachment_id UUID REFERENCES attachments(id) ON DELETE SET NULL,

    post_number INT NOT NULL,
    title VARCHAR NOT NULL,
    content TEXT NOT NULL,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    hidden_at TIMESTAMPTZ,
);
 */

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{AppState, err::AppResult};

#[derive(sqlx::FromRow)]
pub struct DBPost {
    pub id: Uuid,
    pub thread_id: Uuid,
    pub ip_id: Option<Uuid>,
    pub associated_ban_id: Option<Uuid>,
    pub attachment_id: Option<Uuid>,

    pub post_number: i32,
    pub title: String,
    pub content: String,

    pub created_at: DateTime<Utc>,
    pub hidden_at: Option<DateTime<Utc>>,
}


pub struct PostRepository(AppState);

impl PostRepository {
    pub fn new(state: &AppState) -> Self {
        Self(state.clone())
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