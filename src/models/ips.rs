use chrono::{DateTime, Utc};
use uuid::Uuid;
use ipnetwork::IpNetwork;

use crate::{AppState, err::AppResult};

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
}

