use std::default;

use crate::{
    AppState,
    err::AppResult,
    models::{
        admins::Admin,
        boards::{BoardRepository, DbBoard},
    },
};
use chrono::{DateTime, Utc};
use uuid::Uuid;

pub const SUPPORTED_MIME_TYPES: &[&str] = &["image/jpeg", "image/png", "image/gif", "image/webp"];

/* pub fn is_supported_mime_type(attachment_policy: &AttachmentPolicy) -> bool {
    SUPPORTED_MIME_TYPES.contains(&mime_type)
} */

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct DBAttachmentPolicy {
    pub id: Uuid,
    pub board_id: Uuid,
    pub mime_types: Vec<String>,
    pub size_limit: i64,
    pub attachment_limit: i64,
    pub enable_spoilers: bool,
    pub created_at: DateTime<Utc>,
}

impl Default for DBAttachmentPolicy {
    fn default() -> Self {
        DBAttachmentPolicy {
            id: Uuid::default(),
            board_id: Uuid::default(),
            mime_types: SUPPORTED_MIME_TYPES
                .to_vec()
                .iter()
                .map(|mime_type| mime_type.to_string())
                .collect(),
            size_limit: 10485760,
            attachment_limit: 1,
            enable_spoilers: false,
            created_at: DateTime::default(),
        }
    }
}

#[derive(Debug)]
pub struct AttachmentPolicy {
    pub id: Uuid,
    pub board: DbBoard,
    pub mime_types: Vec<String>,
    pub size_limit: i64,
    pub attachment_limit: i64,
    pub enable_spoilers: bool,
    pub created_at: DateTime<Utc>,
}

pub struct CreateAttachmentPolicy {
    pub board_id: Uuid,
    pub mime_types: Vec<String>,
    pub size_limit: Option<i64>,
    pub attachment_limit: Option<i64>,
    pub enable_spoilers: Option<bool>,
}
pub struct EditAttachmentPolicy {
    pub board_id: Option<Uuid>,
    pub mime_types: Option<Vec<String>>,
    pub size_limit: Option<i64>,
    pub attachment_limit: Option<i64>,
    pub enable_spoilers: Option<bool>,
}

pub struct AttachmentPolicyRepository(AppState);

impl AttachmentPolicyRepository {
    pub fn new(state: &AppState) -> Self {
        Self(state.clone())
    }

    pub async fn create(
        &self,
        requestor: Admin,
        create_attachment_policy: CreateAttachmentPolicy,
    ) -> AppResult<DBAttachmentPolicy> {
        tracing::info!(
            "Admin {} is creating a new attachment policy for board: {}",
            requestor.name,
            create_attachment_policy.board_id
        );
        let mut tx = self.0.db.begin().await?;

        let id = sqlx::query_scalar!(
            "INSERT INTO attachment_policies (board_id, mime_types, size_limit, attachment_limit, enable_spoilers) VALUES ($1, $2, $3, $4, $5) RETURNING id",
            create_attachment_policy.board_id,
            &create_attachment_policy.mime_types,
            create_attachment_policy.size_limit.unwrap_or(DBAttachmentPolicy::default().size_limit) as i32,
            create_attachment_policy.attachment_limit.unwrap_or(DBAttachmentPolicy::default().attachment_limit) as i32,
            create_attachment_policy.enable_spoilers.unwrap_or(DBAttachmentPolicy::default().enable_spoilers),
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        let attachment_policy = self.find_by_id(id).await?;

        Ok(attachment_policy)
    }

    pub async fn edit(
        &self,
        requestor: Admin,
        attachment_policy_id: Uuid,
        edit_attachment_policy: EditAttachmentPolicy,
    ) -> AppResult<DBAttachmentPolicy> {
        tracing::info!(
            "Admin {} is editing attachment policy {}",
            requestor.name,
            attachment_policy_id
        );
        let current_attachment_policy = self.find_by_id(attachment_policy_id).await?;
        let size_limit = edit_attachment_policy
            .size_limit
            .unwrap_or(current_attachment_policy.size_limit);
        let attachment_limit = edit_attachment_policy
            .attachment_limit
            .unwrap_or(current_attachment_policy.attachment_limit);
        let enable_spoilers = edit_attachment_policy
            .enable_spoilers
            .unwrap_or(current_attachment_policy.enable_spoilers);
        let mime_types = edit_attachment_policy
            .mime_types
            .unwrap_or(current_attachment_policy.mime_types);
        let board_id = match edit_attachment_policy.board_id {
            Some(board_id) => board_id,
            None => current_attachment_policy.board_id,
        };
        sqlx::query!(
            "UPDATE attachment_policies SET size_limit = $1, attachment_limit = $2, enable_spoilers = $3, mime_types = $4, board_id = $5 WHERE id = $6",
            size_limit as i32,
            attachment_limit as i32,
            enable_spoilers,
            &mime_types,
            board_id,
            attachment_policy_id
        )
        .execute(&self.0.db)
        .await?;

        self.find_by_id(attachment_policy_id).await
    }

    pub async fn delete(&self, requestor: Admin, attachment_policy_id: Uuid) -> AppResult<()> {
        tracing::info!(
            "Admin {} is deleting attachment policy {}",
            requestor.name,
            attachment_policy_id
        );

        let mut tx = self.0.db.begin().await?;

        let attachment_policy_id = sqlx::query_as!(
            DBAttachmentPolicy,
            "SELECT * FROM attachment_policies WHERE id = $1",
            attachment_policy_id
        )
        .fetch_one(&mut *tx)
        .await?;

        sqlx::query!(
            "DELETE FROM attachment_policies WHERE id = $1",
            attachment_policy_id.id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(())
    }

    pub async fn find_by_id(&self, attachment_policy_id: Uuid) -> AppResult<DBAttachmentPolicy> {
        let mut tx = self.0.db.begin().await?;
        let attachment_policy = sqlx::query_as!(
            DBAttachmentPolicy,
            "SELECT * FROM attachment_policies WHERE id = $1",
            attachment_policy_id
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(attachment_policy)
    }
    pub async fn find_by_board_id(&self, board_id: Uuid) -> AppResult<DBAttachmentPolicy> {
        let mut tx = self.0.db.begin().await?;
        let attachment_policy = sqlx::query_as!(
            DBAttachmentPolicy,
            "SELECT * FROM attachment_policies WHERE board_id = $1",
            board_id
        )
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(attachment_policy)
    }

    pub async fn list_all(&self) -> AppResult<Vec<DBAttachmentPolicy>> {
        sqlx::query_as!(DBAttachmentPolicy, "SELECT * FROM attachment_policies")
            .fetch_all(&self.0.db)
            .await
            .map_err(Into::into)
    }

    pub async fn materialize(&self, raw_policy: DBAttachmentPolicy) -> AppResult<AttachmentPolicy> {
        let board_repo = BoardRepository::new(&self.0);
        let board = board_repo.find_by_id(raw_policy.board_id).await?;
        Ok(AttachmentPolicy {
            id: raw_policy.id,
            board,
            mime_types: raw_policy.mime_types,
            size_limit: raw_policy.size_limit,
            attachment_limit: raw_policy.attachment_limit,
            enable_spoilers: raw_policy.enable_spoilers,
            created_at: raw_policy.created_at,
        })
    }
}
