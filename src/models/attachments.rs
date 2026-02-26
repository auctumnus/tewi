use crate::{
    AppState, config,
    err::{AppResult, malformed},
    models::admins::Admin,
};
use chrono::{DateTime, Utc};
use image::{GenericImageView, ImageFormat, ImageReader, imageops::FilterType};
use std::io::Cursor;
use uuid::Uuid;

const MAX_ATTACHMENT_SIZE: usize = 10 * 1024 * 1024; // 10 MB

const SUPPORTED_MIME_TYPES: &[&str] = &["image/jpeg", "image/png", "image/gif", "image/webp"];

const THUMBNAIL_MAX_WIDTH: u32 = 300;
const THUMBNAIL_MAX_HEIGHT: u32 = 300;

fn is_supported_mime_type(mime_type: &str) -> bool {
    SUPPORTED_MIME_TYPES.contains(&mime_type)
}

#[derive(sqlx::FromRow)]
pub struct DBAttachment {
    pub id: Uuid,
    pub post_id: Uuid,
    pub mime_type: String,
    pub size: i64,
    pub width: Option<i64>,
    pub height: Option<i64>,
    pub thumbnail_width: Option<i64>,
    pub thumbnail_height: Option<i64>,
    pub original_filename: String,
    pub spoilered: bool,
    pub removed_at: Option<DateTime<Utc>>,
}
#[derive(Debug)]
pub struct Attachment {
    pub id: Uuid,
    pub post_id: Uuid,
    pub mime_type: String,
    pub size: u32,
    pub dimensions: Option<(u32, u32)>,
    pub thumbnail_dimensions: Option<(u32, u32)>,
    pub original_filename: String,
    pub spoilered: bool,
    pub removed_at: Option<DateTime<Utc>>,
}

pub fn attachment_path(id: &Uuid) -> std::path::PathBuf {
    let config = &config::CONFIG;
    let id_str = id.to_string();
    let (prefix, _) = id_str.split_at(2);
    config.attachments_folder.join(prefix).join(id_str)
}

pub fn thumbnail_path(id: &Uuid) -> std::path::PathBuf {
    let config = &config::CONFIG;
    let id_str = id.to_string();
    let (prefix, _) = id_str.split_at(2);
    config.thumbnails_folder.join(prefix).join(id_str)
}

impl From<DBAttachment> for Attachment {
    fn from(db_attachment: DBAttachment) -> Self {
        let dimensions = match (db_attachment.width, db_attachment.height) {
            (Some(w), Some(h)) => Some((w as u32, h as u32)),
            _ => None,
        };
        let thumbnail_dimensions = match (
            db_attachment.thumbnail_width,
            db_attachment.thumbnail_height,
        ) {
            (Some(w), Some(h)) => Some((w as u32, h as u32)),
            _ => None,
        };
        Attachment {
            id: db_attachment.id,
            post_id: db_attachment.post_id,
            mime_type: db_attachment.mime_type,
            size: db_attachment.size as u32,
            dimensions,
            thumbnail_dimensions,
            original_filename: db_attachment.original_filename,
            spoilered: db_attachment.spoilered,
            removed_at: db_attachment.removed_at,
        }
    }
}

pub struct CreateAttachment {
    pub data: Vec<u8>,
    pub post_id: Uuid,
    pub mime_type: String,
    pub original_filename: String,
    pub spoilered: bool,
}

pub struct AttachmentRepository(AppState);

impl AttachmentRepository {
    pub fn new(state: &AppState) -> Self {
        Self(state.clone())
    }

    pub async fn create(
        &self,
        create_attachment: CreateAttachment,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> AppResult<Attachment> {
        if create_attachment.data.len() > MAX_ATTACHMENT_SIZE {
            return Err(malformed("Attachment size exceeds the maximum allowed"));
        }

        let image = ImageReader::new(Cursor::new(&create_attachment.data)).with_guessed_format()?;

        let guessed_format = image.format();
        let mime_type = ImageFormat::from_mime_type(&create_attachment.mime_type);
        if mime_type.is_none() {
            return Err(malformed("Invalid MIME type"));
        }
        if guessed_format != mime_type {
            return Err(malformed("MIME type does not match image format"));
        }
        if !is_supported_mime_type(&create_attachment.mime_type) {
            return Err(malformed("Unsupported MIME type"));
        }

        let image = image.decode()?;

        let dimensions = image.dimensions();

        let thumbnail = image.resize(
            THUMBNAIL_MAX_WIDTH,
            THUMBNAIL_MAX_HEIGHT,
            FilterType::Gaussian,
        );
        let thumbnail_dimensions = thumbnail.dimensions();

        let id = sqlx::query_scalar!(
            "INSERT INTO attachments (post_id, mime_type, size, width, height, thumbnail_width, thumbnail_height, original_filename, spoilered) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING id",
            create_attachment.post_id,
            create_attachment.mime_type,
            create_attachment.data.len() as i32,
            dimensions.0 as i32,
            dimensions.1 as i32,
            thumbnail_dimensions.0 as i32,
            thumbnail_dimensions.1 as i32,
            create_attachment.original_filename,
            create_attachment.spoilered
        )
        .fetch_one(&mut **tx)
        .await?;

        let attachment_path = attachment_path(&id);
        let thumbnail_path = thumbnail_path(&id);

        tokio::fs::create_dir_all(attachment_path.parent().unwrap()).await?;
        tokio::fs::create_dir_all(thumbnail_path.parent().unwrap()).await?;

        tokio::fs::write(&attachment_path, &create_attachment.data).await?;
        thumbnail.save_with_format(&thumbnail_path, ImageFormat::WebP)?;

        Ok(Attachment {
            id,
            post_id: create_attachment.post_id,
            mime_type: create_attachment.mime_type,
            size: create_attachment.data.len() as u32,
            dimensions: Some(dimensions),
            thumbnail_dimensions: Some(thumbnail_dimensions),
            original_filename: create_attachment.original_filename,
            spoilered: create_attachment.spoilered,
            removed_at: None,
        })
    }

    pub async fn find_by_post_id(&self, post_id: Uuid) -> AppResult<Vec<Attachment>> {
        sqlx::query!("SELECT * FROM attachments WHERE post_id = $1", post_id)
            .fetch_all(&self.0.db)
            .await
            .map(|rows| {
                rows.into_iter()
                    .map(|row| DBAttachment {
                        id: row.id,
                        post_id: row.post_id,
                        mime_type: row.mime_type,
                        size: row.size as i64,
                        width: row.width.map(|w| w as i64),
                        height: row.height.map(|h| h as i64),
                        thumbnail_width: row.thumbnail_width.map(|w| w as i64),
                        thumbnail_height: row.thumbnail_height.map(|h| h as i64),
                        original_filename: row.original_filename,
                        spoilered: row.spoilered,
                        removed_at: row.removed_at,
                    })
                    .collect::<Vec<_>>()
            })
            .map(|db_attachments| db_attachments.into_iter().map(Attachment::from).collect())
            .map_err(Into::into)
    }

    pub async fn delete(&self, requestor: Admin, attachment_id: Uuid) -> AppResult<()> {
        tracing::info!(
            "Admin {} is deleting attachment {}",
            requestor.name,
            attachment_id
        );
        let mut tx = self.0.db.begin().await?;

        sqlx::query!("SELECT * FROM attachments WHERE id = $1", attachment_id)
            .fetch_one(&mut *tx)
            .await?;

        sqlx::query!("DELETE FROM attachments WHERE id = $1", attachment_id)
            .execute(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(())
    }
    pub async fn remove(&self, requestor: Admin, attachment_id: Uuid) -> AppResult<()> {
        tracing::info!(
            "Admin {} is deleting attachment {}",
            requestor.name,
            attachment_id
        );
        let mut tx = self.0.db.begin().await?;

        let attachment = sqlx::query!("SELECT * FROM attachments WHERE id = $1", attachment_id)
            .fetch_one(&mut *tx)
            .await?;

        sqlx::query!(
            "UPDATE attachments SET removed_at = NOW() WHERE id = $1",
            attachment_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        let thumb_file = thumbnail_path(&attachment.id);
        let attachment_file = attachment_path(&attachment.id);

        tokio::fs::remove_file(thumb_file).await?;
        tokio::fs::remove_file(attachment_file).await?;

        Ok(())
    }
    pub async fn remove_post_attachments(&self, requestor: Admin, post_id: Uuid) -> AppResult<()> {
        tracing::info!(
            "Admin {} is deleting attachment(s) for post {}",
            requestor.name,
            post_id
        );
        let mut tx = self.0.db.begin().await?;

        let attachment = sqlx::query!("SELECT * FROM attachments WHERE post_id = $1", post_id)
            .fetch_one(&mut *tx)
            .await?;

        sqlx::query!(
            "UPDATE attachments SET removed_at = NOW() WHERE post_id = $1",
            post_id
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        let thumb_file = thumbnail_path(&attachment.id);
        let attachment_file = attachment_path(&attachment.id);

        tokio::fs::remove_file(thumb_file).await?;
        tokio::fs::remove_file(attachment_file).await?;

        Ok(())
    }
}
