use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{AppState, err::AppResult};

#[derive(sqlx::FromRow)]
pub struct Thread {
    pub id: Uuid,
    pub board_id: Option<Uuid>,
    pub op_post: Uuid,
    pub last_post_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub is_sticky: bool,
}

pub struct ThreadRepository(AppState);

impl ThreadRepository {
    pub fn new(state: &AppState) -> Self {
        Self(state.clone())
    }

    pub async fn find_by_id(&self, thread_id: Uuid) -> AppResult<Thread> {
        sqlx::query_as!(
            Thread,
            "SELECT * FROM threads WHERE id = $1",
            thread_id
        )
        .fetch_one(&self.0.db)
        .await
        .map_err(Into::into)
    }

    pub async fn list_by_board(&self, board_id: Uuid, limit: i64) -> AppResult<Vec<Thread>> {
        sqlx::query_as!(
            Thread,
            "SELECT * FROM threads WHERE board_id = $1 ORDER BY is_sticky DESC, last_post_at DESC LIMIT $2",
            board_id,
            limit
        )
        .fetch_all(&self.0.db)
        .await
        .map_err(Into::into)
    }
}
