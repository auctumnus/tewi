use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{AppState, err::AppResult};

#[derive(sqlx::FromRow)]
pub struct Ban {
    pub id: Uuid,
    pub ip_id: Option<Uuid>,
    pub reason: String,
    pub banned_at: DateTime<Utc>,
    pub banned_by: Option<Uuid>,
    pub expires_at: Option<DateTime<Utc>>,
}

pub struct BanRepository(AppState);

impl BanRepository {
    pub fn new(state: &AppState) -> Self {
        Self(state.clone())
    }

    pub async fn find_by_id(&self, ban_id: Uuid) -> AppResult<Ban> {
        sqlx::query_as!(
            Ban,
            "SELECT * FROM bans WHERE id = $1",
            ban_id
        )
        .fetch_one(&self.0.db)
        .await
        .map_err(Into::into)
    }

    pub async fn find_active_by_ip(&self, ip_id: Uuid) -> AppResult<Option<Ban>> {
        sqlx::query_as!(
            Ban,
            "SELECT * FROM bans WHERE ip_id = $1 AND (expires_at IS NULL OR expires_at > NOW()) ORDER BY banned_at DESC LIMIT 1",
            ip_id
        )
        .fetch_optional(&self.0.db)
        .await
        .map_err(Into::into)
    }
}
