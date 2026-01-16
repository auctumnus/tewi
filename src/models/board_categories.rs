use uuid::Uuid;

use crate::{AppState, err::AppResult};

#[derive(sqlx::FromRow)]
pub struct BoardCategory {
    pub id: Uuid,
    pub name: String,
}

pub struct BoardCategoryRepository(AppState);

impl BoardCategoryRepository {
    pub fn new(state: &AppState) -> Self {
        Self(state.clone())
    }

    pub async fn find_by_id(&self, category_id: Uuid) -> AppResult<BoardCategory> {
        sqlx::query_as!(
            BoardCategory,
            "SELECT * FROM board_categories WHERE id = $1",
            category_id
        )
        .fetch_one(&self.0.db)
        .await
        .map_err(Into::into)
    }

    pub async fn list_all(&self) -> AppResult<Vec<BoardCategory>> {
        sqlx::query_as!(
            BoardCategory,
            "SELECT * FROM board_categories ORDER BY name"
        )
        .fetch_all(&self.0.db)
        .await
        .map_err(Into::into)
    }
}
