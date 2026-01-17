use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Json},
};
use serde::{Deserialize, Serialize};

use crate::AppState;

type PaginationSize = i64;

const MAX_PAGE_SIZE: PaginationSize = 100;

#[derive(Serialize, Debug, Clone)]
pub struct PaginatedResponse<T> {
    pub items: Vec<T>,
    pub total: i64,
    pub offset: PaginationSize,
    pub limit: PaginationSize,
    pub has_more: bool,
}

impl<T> PaginatedResponse<T> {
    pub fn request_last_page(&self) -> PaginatedRequest {
        let last_offset = if self.total % self.limit == 0 {
            self.total - self.limit
        } else {
            self.total - (self.total % self.limit)
        };
        PaginatedRequest {
            limit: self.limit,
            offset: last_offset.try_into().unwrap_or(0),
        }
    }

    pub fn total_pages(&self) -> PaginationSize {
        ((self.total + self.limit - 1) / self.limit).try_into().unwrap_or(0)
    }

    pub fn current_page(&self) -> PaginationSize {
        (self.offset / self.limit) + 1
    }

    pub fn results_text(&self) -> String {
        if self.total == 1 {
            format!("{} result found", self.total)
        } else {
            format!("{} results found", self.total)
        }
    }
}

#[derive(Deserialize, Debug, Clone, Serialize)]
pub struct PaginatedRequest {
    #[serde(default = "default_limit")]
    pub limit: PaginationSize,
    #[serde(default)]
    pub offset: PaginationSize,
}

impl PaginatedRequest {
    pub fn with_previous_page(&self) -> Self {
        let new_offset = (self.offset - self.limit).max(0);
        Self {
            limit: self.limit,
            offset: new_offset,
        }
    }

    pub fn with_next_page(&self) -> Self {
        Self {
            limit: self.limit,
            offset: self.offset + self.limit,
        }
    }
}

fn default_limit() -> PaginationSize {
    10
}

impl Default for PaginatedRequest {
    fn default() -> Self {
        Self {
            limit: default_limit(),
            offset: 0,
        }
    }
}

impl FromRequestParts<AppState> for PaginatedRequest {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let query = parts.uri.query().unwrap_or("");
        let paginated_request: PaginatedRequest = serde_urlencoded::from_str(query)
            .map_err(|_| (StatusCode::BAD_REQUEST, "invalid pagination parameters"))?;

        if paginated_request.limit <= 0 || paginated_request.limit > MAX_PAGE_SIZE {
            return Err((StatusCode::BAD_REQUEST, "invalid limit parameter"));
        }

        if paginated_request.offset < 0 {
            return Err((StatusCode::BAD_REQUEST, "invalid offset parameter"));
        }

        Ok(paginated_request)
    }
}

impl<T: Serialize> IntoResponse for PaginatedResponse<T> {
    fn into_response(self) -> axum::response::Response {
        Json(self).into_response()
    }
}