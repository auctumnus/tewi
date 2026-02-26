use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::models::admins::Admin;
use crate::models::posts::DBPost;
use crate::{
    AppState,
    err::AppResult,
    models::posts::{Post, PostRepository},
};

#[derive(sqlx::FromRow)]
pub struct DBThread {
    pub id: Uuid,
    pub board_id: Option<Uuid>,
    pub op_post: Option<Uuid>,
    pub last_post_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub stickied_at: Option<DateTime<Utc>>,
    pub hidden_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
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
    pub hidden_at: Option<DateTime<Utc>>,
    pub closed_at: Option<DateTime<Utc>>,
}

pub struct AddOpTemplate {
    pub post_id: Uuid,
}

pub enum AdminAction {
    Sticky(bool),
    Close(bool),
    Hide(bool),
}

pub enum ThreadAction {
    AdminAction {
        requestor: Admin,
        action: AdminAction,
    },
}

pub struct ThreadRepository(AppState);

impl ThreadRepository {
    pub fn new(state: &AppState) -> Self {
        Self(state.clone())
    }

    pub async fn create(
        &self,
        board_id: Option<Uuid>,
        op_post: Option<Post>,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> AppResult<DBThread> {
        sqlx::query_as!(
            DBThread,
            "INSERT INTO threads (id, board_id, op_post, last_post_at, created_at, stickied_at, closed_at) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *",
            Uuid::new_v4(),
            board_id,
            op_post.as_ref().map(|p| p.id),
            Utc::now(),
            Utc::now(),
            None::<DateTime<Utc>>,
            None::<DateTime<Utc>>
        )
        .fetch_one(&mut **tx)
        .await
        .map_err(Into::into)
    }

    pub async fn add_op(
        &self,
        thread_id: Uuid,
        add_op: AddOpTemplate,
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    ) -> AppResult<DBThread> {
        sqlx::query_as!(
            DBThread,
            "UPDATE threads SET op_post = $1 WHERE id = $2 returning *",
            add_op.post_id,
            thread_id
        )
        .fetch_one(&mut **tx)
        .await
        .map_err(Into::into)
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
            "SELECT t.* FROM threads t WHERE t.board_id = $1 AND t.op_post = (SELECT p.id FROM posts p JOIN threads th ON p.thread_id = th.id WHERE th.board_id = $1 AND p.post_number = $2)",
            board_id,
            op_post_number
        )
        .fetch_one(&self.0.db)
        .await
        .map_err(Into::into)
    }

    pub async fn find_thread_for_post(&self, post_id: Uuid) -> AppResult<DBThread> {
        sqlx::query_as!(
            DBThread,
            "SELECT t.* FROM threads t JOIN posts p ON t.id = p.thread_id WHERE p.id = $1",
            post_id
        )
        .fetch_one(&self.0.db)
        .await
        .map_err(Into::into)
    }

    pub async fn posts_for_thread(
        &self,
        thread_id: Uuid,
        limit: Option<i64>,
    ) -> AppResult<Vec<Post>> {
        let db_posts = match limit {
            Some(limit) => {
                sqlx::query_as!(
                    DBPost,
                    r#"SELECT
                        *
                    FROM posts
                    WHERE thread_id = $1
                    ORDER BY post_number ASC
                    LIMIT $2 "#,
                    thread_id,
                    limit,
                )
                .fetch_all(&self.0.db)
                .await?
            }
            None => {
                sqlx::query_as!(
                    DBPost,
                    r#"SELECT
                        *
                    FROM posts
                    WHERE thread_id = $1
                    ORDER BY post_number ASC "#,
                    thread_id,
                )
                .fetch_all(&self.0.db)
                .await?
            }
        };

        let post_repo = PostRepository::new(&self.0);
        let mut posts = Vec::with_capacity(db_posts.len());
        for db_post in db_posts {
            posts.push(post_repo.materialize(db_post).await?);
        }
        Ok(posts)
    }

    pub async fn materialize(
        &self,
        db_thread: DBThread,
        reply_limit: Option<i64>,
    ) -> AppResult<Thread> {
        let post_repo = PostRepository::new(&self.0);
        let op_post = if let Some(op_post_id) = db_thread.op_post {
            let op_post = post_repo.find_by_id(op_post_id).await?;
            Some(post_repo.materialize(op_post).await?)
        } else {
            None
        };

        println!("Materializing thread {}", db_thread.id); // --- IGNORE ---

        let replies: Vec<Post> = self
            .posts_for_thread(db_thread.id, reply_limit)
            .await?
            .into_iter()
            .filter_map(|reply| {
                return match &op_post {
                    Some(op_post) => {
                        if reply.id != op_post.id {
                            return Some(reply);
                        } else {
                            return None;
                        }
                    }
                    None => None,
                };
            })
            .collect();
        Ok(Thread {
            id: db_thread.id,
            board_id: db_thread.board_id,
            last_post_at: db_thread.last_post_at,
            created_at: db_thread.created_at,
            stickied_at: db_thread.stickied_at,
            hidden_at: db_thread.hidden_at,
            closed_at: db_thread.closed_at,
            op_post,
            replies,
        })
    }

    pub async fn perform_action(&self, action: ThreadAction, thread_id: Uuid) -> AppResult<()> {
        match action {
            ThreadAction::AdminAction { requestor, action } => match action {
                AdminAction::Sticky(sticky) => {
                    let (verb, query) = match sticky {
                        true => (
                            "stickying",
                            sqlx::query("UPDATE threads SET stickied_at = NOW() WHERE id = $1"),
                        ),
                        false => (
                            "unsticking",
                            sqlx::query("UPDATE threads SET stickied_at = NULL WHERE id = $1"),
                        ),
                    };
                    tracing::info!("Admin {} is {} post {}", requestor.name, verb, thread_id);
                    query.bind(thread_id).execute(&self.0.db).await?
                }
                AdminAction::Hide(hide) => {
                    let (verb, query) = match hide {
                        true => (
                            "hidding",
                            sqlx::query("UPDATE threads SET hidden_at = NOW() WHERE id = $1"),
                        ),
                        false => (
                            "unhiding",
                            sqlx::query("UPDATE threads SET hidden_at = NULL WHERE id = $1"),
                        ),
                    };
                    tracing::info!("Admin {} is {} post {}", requestor.name, verb, thread_id);
                    query.bind(thread_id).execute(&self.0.db).await?
                }
                AdminAction::Close(close) => {
                    let (verb, query) = match close {
                        true => (
                            "closing",
                            sqlx::query("UPDATE threads SET closed_at = NOW() WHERE id = $1"),
                        ),
                        false => (
                            "reopening",
                            sqlx::query("UPDATE threads SET closed_at = NULL WHERE id = $1"),
                        ),
                    };
                    tracing::info!("Admin {} is {} post {}", requestor.name, verb, thread_id);
                    query.bind(thread_id).execute(&self.0.db).await?
                }
            },
        };
        Ok(())
    }
}
