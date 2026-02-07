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
        let last_page = self.total / self.limit;
        PaginatedRequest {
            limit: self.limit,
            page: last_page.try_into().unwrap_or(0),
        }
    }

    pub fn total_pages(&self) -> PaginationSize {
        ((self.total + self.limit - 2) / self.limit)
            .try_into()
            .unwrap_or(0)
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
    #[serde(default = "default_page")]
    pub page: PaginationSize,
}

impl PaginatedRequest {
    pub fn with_previous_page(&self) -> Self {
        let new_page = (self.page - 1).max(1);
        Self {
            limit: self.limit,
            page: new_page,
        }
    }

    pub fn with_next_page(&self) -> Self {
        Self {
            limit: self.limit,
            page: self.page + 1,
        }
    }

    pub fn current_offset(&self) -> i64 {
        (self.page - 1) * self.limit
    }
}

fn default_limit() -> PaginationSize {
    10
}
fn default_page() -> PaginationSize {
    1
}

impl Default for PaginatedRequest {
    fn default() -> Self {
        Self {
            limit: default_limit(),
            page: default_page(),
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

        if paginated_request.page < 1 {
            return Err((StatusCode::BAD_REQUEST, "invalid page parameter"));
        }

        Ok(paginated_request)
    }
}

impl<T: Serialize> IntoResponse for PaginatedResponse<T> {
    fn into_response(self) -> axum::response::Response {
        Json(self).into_response()
    }
}
