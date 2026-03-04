use std::{collections::HashMap, iter::Map};

use uuid::Uuid;

use crate::{
    AppState,
    err::AppResult,
    models::{
        admins::Admin,
        attachment_policies::{AttachmentPolicy, AttachmentPolicyRepository, DBAttachmentPolicy},
        board_categories,
        threads::{DBThread, Thread, ThreadRepository},
    },
    pagination::{PaginatedRequest, PaginatedResponse},
};

#[derive(sqlx::FromRow, Debug, Clone)]
pub struct DbBoard {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub category_id: Option<Uuid>,
    pub next_post_number: i32,
}
#[derive(sqlx::FromRow, Debug, Clone)]
pub struct Board {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub category_id: Option<Uuid>,
    pub next_post_number: i32,
    pub attachment_policy: DBAttachmentPolicy,
}

#[derive(Debug)]
pub struct BoardByCategory(pub Option<String>, pub Vec<DbBoard>);

#[derive(sqlx::FromRow, Debug)]
struct BoardWithCategoryName {
    pub id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub category_id: Option<Uuid>,
    pub category_name: Option<String>,
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

    pub async fn create(&self, requestor: Admin, create_board: CreateBoard) -> AppResult<DbBoard> {
        tracing::info!(
            "Admin {} is creating a new board: /{}/",
            requestor.name,
            create_board.slug
        );
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
        sqlx::query!("DELETE FROM boards WHERE id = $1", board_id)
            .execute(&self.0.db)
            .await?;
        Ok(())
    }

    pub async fn edit(
        &self,
        requestor: Admin,
        board_id: Uuid,
        edit_board: EditBoard,
    ) -> AppResult<DbBoard> {
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

    pub async fn find_by_id(&self, board_id: Uuid) -> AppResult<DbBoard> {
        sqlx::query_as!(DbBoard, "SELECT * FROM boards WHERE id = $1", board_id)
            .fetch_one(&self.0.db)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_slug(&self, slug: &str) -> AppResult<DbBoard> {
        sqlx::query_as!(DbBoard, "SELECT * FROM boards WHERE slug = $1", slug)
            .fetch_one(&self.0.db)
            .await
            .map_err(Into::into)
    }

    pub async fn list_all(&self) -> AppResult<Vec<DbBoard>> {
        sqlx::query_as!(DbBoard, "SELECT * FROM boards ORDER BY name")
            .fetch_all(&self.0.db)
            .await
            .map_err(Into::into)
    }

    pub async fn list_all_slugs(&self) -> AppResult<Vec<String>> {
        return sqlx::query_as!(DbBoard, "SELECT * FROM boards ORDER BY name")
            .fetch_all(&self.0.db)
            .await
            .map_err(Into::into)
            .map(|boards| boards.iter().map(|board| board.slug.clone()).collect());
    }

    pub async fn list_all_category_grouped(&self) -> AppResult<Vec<BoardByCategory>> {
        let boards = sqlx::query_as!(
            BoardWithCategoryName,
            r#"SELECT b.*, c.name as "category_name?"
            FROM boards b
            left JOIN board_categories c
            ON c.id = b.category_id
            order by c.name"#
        )
        .fetch_all(&self.0.db)
        .await
        .map_err(Into::<sqlx::Error>::into)?;

        let mut groups = HashMap::<Option<String>, Vec<DbBoard>>::new();
        for board in boards {
            if let Some(cat) = groups.get_mut(&board.category_name) {
                cat.push(DbBoard {
                    id: board.id,
                    slug: board.slug,
                    name: board.name,
                    description: board.description,
                    category_id: board.category_id,
                    next_post_number: board.next_post_number,
                });
            } else {
                groups.insert(
                    board.category_name,
                    vec![DbBoard {
                        id: board.id,
                        slug: board.slug,
                        name: board.name,
                        description: board.description,
                        category_id: board.category_id,
                        next_post_number: board.next_post_number,
                    }],
                );
            }
        }
        return Ok(groups
            .iter_mut()
            .map(|(key, value)| BoardByCategory(key.clone(), value.clone()))
            .collect());
    }

    pub async fn increment_next_post_number(
        &self,
        board_id: Uuid,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> AppResult<i32> {
        let next_post_number = sqlx::query_scalar!(
            "UPDATE boards SET next_post_number = next_post_number + 1 WHERE id = $1 RETURNING next_post_number",
            board_id
        )
        .fetch_one(&mut **tx)
        .await?;
        Ok(next_post_number)
    }

    pub async fn delete_by_name(&self, requestor: Admin, name: &str) -> AppResult<()> {
        tracing::info!("Admin {} is deleting board {}", requestor.name, name);
        sqlx::query!("DELETE FROM boards WHERE name = $1", name)
            .execute(&self.0.db)
            .await?;
        Ok(())
    }

    pub async fn threads_for_board(
        &self,
        board_id: Uuid,
        pagination: PaginatedRequest,
        ignore_hidden: bool,
    ) -> AppResult<PaginatedResponse<Thread>> {
        let db_threads = match ignore_hidden {
            true => {
                sqlx::query_as!(
                    DBThread,
                    r#"SELECT *
                    FROM threads
                    WHERE board_id = $1 AND hidden_at IS NULL
                    ORDER BY stickied_at ASC, last_post_at DESC
                    LIMIT $2
                    OFFSET $3"#,
                    board_id,
                    pagination.limit,
                    pagination.current_offset()
                )
                .fetch_all(&self.0.db)
                .await?
            }
            false => {
                sqlx::query_as!(
                    DBThread,
                    r#"SELECT *
                    FROM threads
                    WHERE board_id = $1
                    ORDER BY stickied_at ASC, last_post_at DESC
                    LIMIT $2
                    OFFSET $3"#,
                    board_id,
                    pagination.limit,
                    pagination.current_offset()
                )
                .fetch_all(&self.0.db)
                .await?
            }
        };
        let total = match ignore_hidden {
            true => sqlx::query_scalar!(
                "SELECT COUNT(*) FROM threads WHERE board_id = $1 AND hidden_at IS NULL",
                board_id
            )
            .fetch_one(&self.0.db)
            .await?
            .unwrap_or(0),
            false => {
                sqlx::query_scalar!("SELECT COUNT(*) FROM threads WHERE board_id = $1", board_id)
                    .fetch_one(&self.0.db)
                    .await?
                    .unwrap_or(0)
            }
        };

        let thread_repo = ThreadRepository::new(&self.0);
        let mut threads = Vec::with_capacity(db_threads.len());
        for db_thread in db_threads {
            threads.push(thread_repo.materialize(db_thread, Some(2)).await?);
        }
        Ok(PaginatedResponse {
            items: threads,
            total,
            offset: pagination.current_offset(),
            limit: pagination.limit,
            has_more: (pagination.current_offset() + pagination.limit) < total,
        })
    }

    pub async fn materialize(&self, raw_board: DbBoard) -> AppResult<Board> {
        let attachment_policy_repo = AttachmentPolicyRepository::new(&self.0);
        let attachment_policy = attachment_policy_repo
            .find_by_id(raw_board.id)
            .await
            .map(|policy| policy)
            .unwrap_or(DBAttachmentPolicy::default());
        Ok(Board {
            id: raw_board.id,
            slug: raw_board.slug,
            name: raw_board.name,
            description: raw_board.description,
            category_id: raw_board.category_id,
            next_post_number: raw_board.next_post_number,
            attachment_policy,
        })
    }
}
