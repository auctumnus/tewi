use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{AppState, err::AppResult};

#[derive(sqlx::FromRow)]
pub struct Admin {
    pub id: Uuid,
    pub name: String,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
}

pub struct AdminRepository(AppState);

impl AdminRepository {
    pub fn new(state: &AppState) -> Self {
        Self(state.clone())
    }

    pub async fn find_by_id(&self, admin_id: Uuid) -> AppResult<Admin> {
        sqlx::query_as!(
            Admin,
            "SELECT * FROM admins WHERE id = $1",
            admin_id
        )
        .fetch_one(&self.0.db)
        .await
        .map_err(Into::into)
    }

    pub async fn find_by_name(&self, name: &str) -> AppResult<Admin> {
        sqlx::query_as!(
            Admin,
            "SELECT * FROM admins WHERE name = $1",
            name
        )
        .fetch_one(&self.0.db)
        .await
        .map_err(Into::into)
    }
}
