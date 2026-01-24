use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{AppState, auth::hash, err::AppResult};

#[derive(sqlx::FromRow, Debug)]
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
    
    pub async fn create(&self, name: &str, password: &str) -> AppResult<Admin> {
        let password_hash = hash(password)?;
        let admin = sqlx::query_as!(
            Admin,
            "INSERT INTO admins (name, password_hash) VALUES ($1, $2) RETURNING *",
            name,
            password_hash
        )
        .fetch_one(&self.0.db)
        .await?;
        Ok(admin)
    }

    pub async fn delete_by_name(&self, name: &str) -> AppResult<()> {
        sqlx::query!(
            "DELETE FROM admins WHERE name = $1",
            name
        )
        .execute(&self.0.db)
        .await?;
        Ok(())
    }

    pub async fn change_password(&self, name: &str, new_password: &str) -> AppResult<()> {
        let new_password_hash = hash(new_password)?;
        sqlx::query!(
            "UPDATE admins SET password_hash = $1 WHERE name = $2",
            new_password_hash,
            name
        )
        .execute(&self.0.db)
        .await?;
        Ok(())
    }

    pub async fn list_all(&self) -> AppResult<Vec<Admin>> {
        let admins = sqlx::query_as!(
            Admin,
            "SELECT * FROM admins ORDER BY created_at DESC"
        )
        .fetch_all(&self.0.db)
        .await?;
        Ok(admins)
    }
}