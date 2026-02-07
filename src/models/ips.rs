use chrono::{DateTime, Utc};
use uuid::Uuid;
use ipnetwork::IpNetwork;

use crate::{AppState, err::AppResult, models::posts::{DBPost, Post, PostRepository}, pagination::{PaginatedRequest, PaginatedResponse}};

#[derive(sqlx::FromRow)]
pub struct Ip {
    pub id: Uuid,
    pub ip_address: IpNetwork,
    pub created_at: DateTime<Utc>,
}

pub struct IpRepository(AppState);

impl IpRepository {
    pub fn new(state: &AppState) -> Self {
        Self(state.clone())
    }

    pub async fn find_by_id(&self, ip_id: Uuid) -> AppResult<Ip> {
        sqlx::query_as!(
            Ip,
            "SELECT * FROM ips WHERE id = $1",
            ip_id
        )
        .fetch_one(&self.0.db)
        .await
        .map_err(Into::into)
    }

    pub async fn find_by_address(&self, ip_address: IpNetwork) -> AppResult<Ip> {
        sqlx::query_as!(
            Ip,
            "SELECT * FROM ips WHERE ip_address = $1",
            ip_address
        )
        .fetch_one(&self.0.db)
        .await
        .map_err(Into::into)
    }

    pub async fn find_or_create(&self, ip_address: IpNetwork) -> AppResult<Ip> {
        match self.find_by_address(ip_address).await {
            Ok(ip) => Ok(ip),
            Err(_) => {
                let ip_id = Uuid::new_v4();
                sqlx::query!(
                    "INSERT INTO ips (id, ip_address, created_at) VALUES ($1, $2, $3)",
                    ip_id,
                    ip_address,
                    Utc::now()
                )
                .execute(&self.0.db)
                .await?;
                self.find_by_id(ip_id).await
            }
        }
    }

    pub async fn find_posts_by_id(&self, ip_address: IpNetwork, pagination: PaginatedRequest) -> AppResult<PaginatedResponse<Post>> {
        let ip = self.find_or_create(ip_address).await?;
        let db_posts = sqlx::query_as!(
            DBPost,
            "SELECT * FROM posts WHERE ip_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
            ip.id,
            pagination.limit,
            pagination.current_offset()
        )
        .fetch_all(&self.0.db)
        .await?;
        let total = sqlx::query!(
            "SELECT COUNT(*) as count FROM posts WHERE ip_id = $1",
            ip.id
        )
        .fetch_one(&self.0.db)
        .await?
        .count
        .unwrap_or(0);

        let posts_repo = PostRepository::new(&self.0);
        let mut posts = Vec::with_capacity(db_posts.len());
        for db_post in db_posts {
            let post = posts_repo.materialize(db_post).await?;
            posts.push(post);
        }
        Ok(PaginatedResponse {
            items: posts,
            total,
            offset: pagination.current_offset(),
            limit: pagination.limit,
            has_more: (pagination.current_offset() + pagination.limit) < total,
        })
    }
}