use std::str::FromStr;

use axum::{
    extract::{Request, State},
    http::header::CONTENT_DISPOSITION,
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::{AppState, models::attachments::AttachmentRepository};

pub async fn id_to_original_filename(
    State(s): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let uri = request.uri().clone();
    let mut response = next.run(request).await;

    if !response.status().is_success() {
        return response;
    }

    let Some(raw_id) = uri.path().split("/").last() else {
        tracing::warn!(
            "THIS SHOULD NEVER HAPPEN - Could not find original file name for requeted upload - Problem: No file ID URL"
        );
        return response;
    };

    let Ok(attachment_id) = Uuid::from_str(raw_id) else {
        tracing::warn!(
            "THIS SHOULD NEVER HAPPEN - Could not find original file name for requeted upload - Problem: file ID in URL is not a UUID {raw_id}"
        );
        return response;
    };

    let Ok(attachment) = AttachmentRepository::new(&s)
        .find_by_attachment_id(attachment_id)
        .await
    else {
        tracing::warn!(
            "THIS SHOULD NEVER HAPPEN - Could not find original file name for requeted upload - Problem: Unknown file ID"
        );
        return response;
    };

    let Ok(content_disposition_value) = format!(
        "attachment;filename=\"{}\"",
        attachment.original_filename.as_str()
    )
    .parse() else {
        tracing::warn!(
            "THIS SHOULD NEVER HAPPEN - Could not find original file name for requeted upload - Problem: could not create valid Content-Disposition value from {}",
            attachment.original_filename
        );
        return response;
    };

    response
        .headers_mut()
        .insert(CONTENT_DISPOSITION, content_disposition_value);
    response
}
