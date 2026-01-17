use uuid::Uuid;

use crate::{AppState, err::AppResult, models::{admins::Admin, threads::{DBThread, Thread, ThreadRepository}}, pagination::{PaginatedRequest, PaginatedResponse}};

#[derive(sqlx::FromRow)]
pub struct Board {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub category_id: Option<Uuid>,
    pub next_post_number: i32,
}

pub struct CreateBoard {
    pub slug: String,
    pub name: String,
    pub description: String,
    pub category_id: Option<Uuid>,
}

pub struct EditBoard {
    pub slug: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    // pass None to leave unchanged, Some(None) to remove
    pub category_id: Option<Option<Uuid>>,
}

pub struct BoardRepository(AppState);

impl BoardRepository {
    pub fn new(state: &AppState) -> Self {
        Self(state.clone())
    }

    pub async fn create(&self, requestor: Admin, create_board: CreateBoard) -> AppResult<Board> {
        tracing::info!("Admin {} is creating a new board: /{}/", requestor.name, create_board.slug);
        let board_id = Uuid::new_v4();
        sqlx::query!(
            "INSERT INTO boards (id, slug, name, description, category_id, next_post_number) VALUES ($1, $2, $3, $4, $5, $6)",
            board_id,
            create_board.slug,
            create_board.name,
            create_board.description,
            create_board.category_id,
            1
        )
        .execute(&self.0.db)
        .await?;
        self.find_by_id(board_id).await
    }

    pub async fn delete(&self, requestor: Admin, board_id: Uuid) -> AppResult<()> {
        tracing::info!("Admin {} is deleting board {}", requestor.name, board_id);
        sqlx::query!(
            "DELETE FROM boards WHERE id = $1",
            board_id
        )
        .execute(&self.0.db)
        .await?;
        Ok(())
    }

    pub async fn edit(&self, requestor: Admin, board_id: Uuid, edit_board: EditBoard) -> AppResult<Board> {
        tracing::info!("Admin {} is editing board {}", requestor.name, board_id);
        let current_board = self.find_by_id(board_id).await?;
        let slug = edit_board.slug.unwrap_or(current_board.slug);
        let name = edit_board.name.unwrap_or(current_board.name);
        let description = edit_board.description.unwrap_or(current_board.description);
        let category_id = match edit_board.category_id {
            Some(category_id) => category_id,
            None => current_board.category_id,
        };
        sqlx::query!(
            "UPDATE boards SET slug = $1, name = $2, description = $3, category_id = $4 WHERE id = $5",
            slug,
            name,
            description,
            category_id,
            board_id
        )
        .execute(&self.0.db)
        .await?;
        self.find_by_id(board_id).await
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

    pub async fn increment_next_post_number(&self, board_id: Uuid) -> AppResult<i32> {
        let next_post_number = sqlx::query_scalar!(
            "UPDATE boards SET next_post_number = next_post_number + 1 WHERE id = $1 RETURNING next_post_number",
            board_id
        )
        .fetch_one(&self.0.db)
        .await?;
        Ok(next_post_number)
    }

    pub async fn threads_for_board(&self, board_id: Uuid, pagination: PaginatedRequest) -> AppResult<PaginatedResponse<Thread>> {
        let db_threads = sqlx::query_as!(
            DBThread,
            r#"SELECT *
            FROM threads
            WHERE board_id = $1
            ORDER BY stickied_at DESC, last_post_at DESC
            LIMIT $2
            OFFSET $3"#,
            board_id,
            pagination.limit,
            pagination.offset
        )
        .fetch_all(&self.0.db)
        .await?;
        let total = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM threads WHERE board_id = $1",
            board_id
        )
        .fetch_one(&self.0.db)
        .await?.unwrap_or(0);

        let thread_repo = ThreadRepository::new(&self.0);
        let mut threads = Vec::with_capacity(db_threads.len());
        for db_thread in db_threads {
            threads.push(thread_repo.materialize(db_thread).await?);
        }
        Ok(PaginatedResponse {
            items: threads,
            total,
            offset: pagination.offset,
            limit: pagination.limit,
            has_more: (pagination.offset + pagination.limit) < total,
        })
    }
}