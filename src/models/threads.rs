use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::models::posts::DBPost;
use crate::view_structs::admin::edit_board::EditBoardTemplate;
use crate::{
    AppState,
    err::AppResult,
    models::posts::{Post, PostRepository},
    pagination::{PaginatedRequest, PaginatedResponse},
};

#[derive(sqlx::FromRow)]
pub struct DBThread {
    pub id: Uuid,
    pub board_id: Option<Uuid>,
    pub op_post: Option<Uuid>,
    pub last_post_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub stickied_at: Option<DateTime<Utc>>,
}
#[derive(Debug)]
pub struct Thread {
    pub id: Uuid,
    pub board_id: Option<Uuid>,
    pub op_post: Option<Post>,
    pub last_post_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub stickied_at: Option<DateTime<Utc>>,
    pub replies: Vec<Post>, // latest n replies
}

pub struct AddOpTemplate {
    pub post_id: Uuid,
}

pub struct ThreadRepository(AppState);

impl ThreadRepository {
    pub fn new(state: &AppState) -> Self {
        Self(state.clone())
    }

    pub async fn create(&self, board_id: Option<Uuid>, op_post: Option<Post>) -> AppResult<Thread> {
        let thread_id = sqlx::query_scalar!(
            "INSERT INTO threads (id, board_id, op_post, last_post_at, created_at, stickied_at) VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
            Uuid::new_v4(),
            board_id,
            op_post.as_ref().map(|p| p.id),
            Utc::now(),
            Utc::now(),
            None::<DateTime<Utc>>
        )
        .fetch_one(&self.0.db)
        .await?;

        // Fetch the newly created thread
        let db_thread = self.find_by_id(thread_id).await?;
        self.materialize(db_thread).await
    }

    pub async fn add_op(&self, thread_id: Uuid, add_op: AddOpTemplate) -> AppResult<DBThread> {
        sqlx::query!(
            "UPDATE threads SET op_post = $1 WHERE id = $2",
            add_op.post_id,
            thread_id
        )
        .execute(&self.0.db)
        .await?;
        self.find_by_id(thread_id).await
    }

    pub async fn find_by_id(&self, thread_id: Uuid) -> AppResult<DBThread> {
        sqlx::query_as!(DBThread, "SELECT * FROM threads WHERE id = $1", thread_id)
            .fetch_one(&self.0.db)
            .await
            .map_err(Into::into)
    }

    pub async fn find_by_board_and_number(
        &self,
        board_id: Uuid,
        op_post_number: i32,
    ) -> AppResult<DBThread> {
        sqlx::query_as!(
            DBThread,
            "SELECT * FROM threads WHERE board_id = $1 AND op_post = (SELECT id FROM posts WHERE board_id = $1 AND post_number = $2)",
            board_id,
            op_post_number
        )
        .fetch_one(&self.0.db)
        .await
        .map_err(Into::into)
    }

    pub async fn posts_for_thread(
        &self,
        thread_id: Uuid,
        pagination: PaginatedRequest,
    ) -> AppResult<PaginatedResponse<Post>> {
        let db_posts = sqlx::query_as!(
            DBPost,
            r#"SELECT
                *
            FROM posts
            WHERE thread_id = $1
            ORDER BY post_number ASC
            LIMIT $2
            OFFSET $3 "#,
            thread_id,
            pagination.limit,
            pagination.offset
        )
        .fetch_all(&self.0.db)
        .await?;
        let total =
            sqlx::query_scalar!("SELECT COUNT(*) FROM posts WHERE thread_id = $1", thread_id)
                .fetch_one(&self.0.db)
                .await?
                .unwrap_or(0);

        let post_repo = PostRepository::new(&self.0);
        let mut posts = Vec::with_capacity(db_posts.len());
        for db_post in db_posts {
            posts.push(post_repo.materialize(db_post).await?);
        }
        Ok(PaginatedResponse {
            items: posts,
            total,
            offset: pagination.offset,
            limit: pagination.limit,
            has_more: (pagination.offset + pagination.limit) < total,
        })
    }

    pub async fn materialize(&self, db_thread: DBThread) -> AppResult<Thread> {
        let post_repo = PostRepository::new(&self.0);
        let op_post = if let Some(op_post_id) = db_thread.op_post {
            let op_post = post_repo.find_by_id(op_post_id).await?;
            Some(post_repo.materialize(op_post).await?)
        } else {
            None
        };
        let replies = self
            .posts_for_thread(
                db_thread.id,
                PaginatedRequest {
                    offset: 0,
                    limit: 5,
                },
            )
            .await?
            .items;
        Ok(Thread {
            id: db_thread.id,
            board_id: db_thread.board_id,
            last_post_at: db_thread.last_post_at,
            created_at: db_thread.created_at,
            stickied_at: db_thread.stickied_at,
            op_post,
            replies,
        })
    }
}
