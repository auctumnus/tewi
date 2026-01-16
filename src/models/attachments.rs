use uuid::Uuid;
use chrono::{DateTime, Utc};
use crate::{AppState, err::AppResult};

#[derive(sqlx::FromRow)]
pub struct DBAttachment {
    pub id: Uuid,
    pub mime_type: String,
    pub size: i32,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub thumbnail_width: Option<i32>,
    pub thumbnail_height: Option<i32>,
    pub original_filename: String,
    pub spoilered: bool,
}

pub struct Attachment {
    pub id: Uuid,
    pub mime_type: String,
    pub size: i32,
    pub dimensions: Option<(i32, i32)>,
    pub thumbnail_dimensions: Option<(i32, i32)>,
    pub original_filename: String,
    pub spoilered: bool,
}

impl From<DBAttachment> for Attachment {
    fn from(db_attachment: DBAttachment) -> Self {
        let dimensions = match (db_attachment.width, db_attachment.height) {
            (Some(w), Some(h)) => Some((w, h)),
            _ => None,
        };
        let thumbnail_dimensions = match (db_attachment.thumbnail_width, db_attachment.thumbnail_height) {
            (Some(w), Some(h)) => Some((w, h)),
            _ => None,
        };
        Attachment {
            id: db_attachment.id,
            mime_type: db_attachment.mime_type,
            size: db_attachment.size,
            dimensions,
            thumbnail_dimensions,
            original_filename: db_attachment.original_filename,
            spoilered: db_attachment.spoilered,
        }
    }
}

pub struct AttachmentRepository(AppState);

impl AttachmentRepository {
    pub fn new(state: &AppState) -> Self {
        Self(state.clone())
    }

    pub async fn find_by_id(&self, attachment_id: Uuid) -> AppResult<Attachment> {
        sqlx::query_as!(
            DBAttachment,
            "SELECT * FROM attachments WHERE id = $1",
            attachment_id
        )
        .fetch_one(&self.0.db)
        .await
        .map(Attachment::from)
        .map_err(Into::into)
    }
}
