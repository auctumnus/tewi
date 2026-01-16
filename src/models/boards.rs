use uuid::Uuid;

use crate::{AppState, err::AppResult};

#[derive(sqlx::FromRow)]
pub struct Board {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub category_id: Option<Uuid>,
}

pub struct BoardRepository(AppState);

impl BoardRepository {
    pub fn new(state: &AppState) -> Self {
        Self(state.clone())
    }

    pub async fn find_by_id(&self, board_id: Uuid) -> AppResult<Board> {
        sqlx::query_as!(
            Board,
            "SELECT * FROM boards WHERE id = $1",
            board_id
        )
        .fetch_one(&self.0.db)
        .await
        .map_err(Into::into)
    }

    pub async fn find_by_slug(&self, slug: &str) -> AppResult<Board> {
        sqlx::query_as!(
            Board,
            "SELECT * FROM boards WHERE slug = $1",
            slug
        )
        .fetch_one(&self.0.db)
        .await
        .map_err(Into::into)
    }

    pub async fn list_all(&self) -> AppResult<Vec<Board>> {
        sqlx::query_as!(
            Board,
            "SELECT * FROM boards ORDER BY name"
        )
        .fetch_all(&self.0.db)
        .await
        .map_err(Into::into)
    }
}
