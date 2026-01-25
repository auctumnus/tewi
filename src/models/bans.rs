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

#[derive(sqlx::FromRow)]
pub struct BanListEntry {
    pub id: Uuid,
    pub reason: String,
    pub banned_at: DateTime<Utc>,
    pub admin_name: Option<String>,
    pub post_number: Option<i32>,
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

    pub async fn list_all(&self) -> AppResult<Vec<Ban>> {
        sqlx::query_as!(Ban, "SELECT * FROM bans ORDER BY banned_at DESC")
            .fetch_all(&self.0.db)
            .await
            .map_err(Into::into)
    }

    pub async fn materialize(&self, ban: Ban) -> AppResult<BanListEntry> {
        let admin_name = match ban.banned_by {
            Some(admin_id) => {
                sqlx::query_scalar!("SELECT name FROM admins WHERE id = $1", admin_id)
                    .fetch_optional(&self.0.db)
                    .await?
            }
            None => None,
        };

        let post_number = sqlx::query_scalar!(
            "SELECT post_number FROM posts WHERE associated_ban_id = $1",
            ban.id
        )
        .fetch_optional(&self.0.db)
        .await?;

        Ok(BanListEntry {
            id: ban.id,
            reason: ban.reason,
            banned_at: ban.banned_at,
            admin_name,
            post_number,
        })
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
