use uuid::Uuid;

use crate::{AppState, err::AppResult, models::{admins::Admin, boards::Board}};

#[derive(sqlx::FromRow)]
pub struct DBBoardCategory {
    pub id: Uuid,
    pub name: String,
}

pub struct BoardCategory {
    pub id: Uuid,
    pub name: String,
    pub boards: Vec<Board>,
}

pub struct EditBoardCategory {
    pub name: Option<String>,
}

pub struct BoardCategoryRepository(AppState);

impl BoardCategoryRepository {
    pub fn new(state: &AppState) -> Self {
        Self(state.clone())
    }

    pub async fn create(&self, requestor: Admin, name: String) -> AppResult<DBBoardCategory> {
        tracing::info!("Admin {} is creating a new board category: {}", requestor.name, name);
        let category_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO board_categories (id, name) VALUES ($1, $2)",
            category_id,
            name
        )
        .execute(&self.0.db)
        .await?;
        self.find_by_id(category_id).await
    }

    pub async fn edit(&self, requestor: Admin, category_id: Uuid, edit_category: EditBoardCategory) -> AppResult<DBBoardCategory> {
        tracing::info!("Admin {} is editing board category {}", requestor.name, category_id);
        let current_category = self.find_by_id(category_id).await?;
        let name = edit_category.name.unwrap_or(current_category.name);
        sqlx::query!(
            "UPDATE board_categories SET name = $1 WHERE id = $2",
            name,
            category_id
        )
        .execute(&self.0.db)
        .await?;
        self.find_by_id(category_id).await
    }

    pub async fn delete(&self, requestor: Admin, category_id: Uuid) -> AppResult<()> {
        tracing::info!("Admin {} is deleting board category {}", requestor.name, category_id);
        sqlx::query!(
            "DELETE FROM board_categories WHERE id = $1",
            category_id
        )
        .execute(&self.0.db)
        .await?;
        Ok(())
    }

    pub async fn find_by_id(&self, category_id: Uuid) -> AppResult<DBBoardCategory> {
        sqlx::query_as!(
            DBBoardCategory,
            "SELECT * FROM board_categories WHERE id = $1",
            category_id
        )
        .fetch_one(&self.0.db)
        .await
        .map_err(Into::into)
    }

    pub async fn materialize(&self, db_category: DBBoardCategory) -> AppResult<BoardCategory> {
        let boards = sqlx::query_as!(
            Board,
            "SELECT * FROM boards WHERE category_id = $1 ORDER BY name",
            db_category.id
        )
        .fetch_all(&self.0.db)
        .await?;
        Ok(BoardCategory {
            id: db_category.id,
            name: db_category.name,
            boards,
        })
    }

    pub async fn list_all(&self) -> AppResult<Vec<BoardCategory>> {
        let db_board_categories = sqlx::query_as!(
            DBBoardCategory,
            "SELECT * FROM board_categories ORDER BY name"
        )
        .fetch_all(&self.0.db)
        .await?;

        let mut board_categories = Vec::new();
        for db_category in db_board_categories {
            let category = self.materialize(db_category).await?;
            board_categories.push(category);
        }
        Ok(board_categories)
    }
}