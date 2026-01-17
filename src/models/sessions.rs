use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{AppState, auth::{hash, verify}, err::{AppResult, invalid_credentials}, models::admins::AdminRepository};

#[derive(sqlx::FromRow, Debug)]
pub struct Session {
    pub id: i32,
    pub admin_id: Uuid,
    pub token: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

pub struct SessionRepository(AppState);

impl SessionRepository {
    pub fn new(state: &AppState) -> Self {
        Self(state.clone())
    }

    pub async fn find_by_token(&self, token: &str) -> AppResult<Option<Session>> {
        sqlx::query_as!(
            Session,
            "SELECT * FROM sessions WHERE token = $1",
            token
        )
        .fetch_optional(&self.0.db)
        .await
        .map_err(Into::into)
    }

    pub async fn create(&self, name: &str, password: &str) -> AppResult<Session> {
        let admins = AdminRepository::new(&self.0);
        let admin = admins.find_by_name(name).await?;

        if verify(password, &admin.password_hash)? {
            // Password is correct, create a new session
            let token = Uuid::new_v4().to_string();
            let expires_at = Utc::now() + chrono::Duration::hours(1);
            let session = sqlx::query_as!(
                Session,
                "INSERT INTO sessions (admin_id, token, expires_at) VALUES ($1, $2, $3) RETURNING *",
                admin.id,
                token,
                expires_at
            )
            .fetch_one(&self.0.db)
            .await?;
            Ok(session)
        } else {
            Err(invalid_credentials())
        }
    }

    pub async fn delete_by_token(&self, token: &str) -> AppResult<()> {
        sqlx::query!(
            "DELETE FROM sessions WHERE token = $1",
            token
        )
        .execute(&self.0.db)
        .await?;
        Ok(())
    }

    pub async fn delete_expired(&self) -> AppResult<()> {
        sqlx::query!(
            "DELETE FROM sessions WHERE expires_at < NOW()"
        )
        .execute(&self.0.db)
        .await?;
        Ok(())
    }
}
