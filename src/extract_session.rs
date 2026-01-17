use axum::{
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use axum_extra::extract::CookieJar;

use crate::models::{
    admins::{Admin, AdminRepository}, sessions::{Session, SessionRepository}
};

use super::AppState;

pub struct AdminSession(pub Option<(Session, Admin)>);

impl AdminSession {
    #[allow(dead_code)]
    pub const fn is_authenticated(&self) -> bool {
        self.0.is_some()
    }

    pub fn admin(&self) -> Option<&Admin> {
        self.0.as_ref().map(|(_, admin)| admin)
    }

    pub fn session(&self) -> Option<&Session> {
        self.0.as_ref().map(|(session, _)| session)
    }
}

pub const SESSION_COOKIE_NAME: &str = "session_token";

fn token_from_header(parts: &Parts) -> Option<String> {
    parts
        .headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(str::to_string)
}

async fn token_from_jar(parts: &mut Parts) -> Option<String> {
    CookieJar::from_request_parts(parts, &())
        .await
        .ok()
        .and_then(|jar| {
            jar.get(SESSION_COOKIE_NAME)
                .map(|cookie| cookie.value().to_string())
        })
}

impl FromRequestParts<AppState> for AdminSession {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let session_repo = SessionRepository::new(state);

        // async_flatmap save me

        let session_token = match token_from_header(parts) {
            Some(token) => Some(token),
            None => token_from_jar(parts).await,
        };

        let session = match session_token {
            Some(token) => session_repo
                .find_by_token(&token)
                .await
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to find session"))?,
            None => None,
        };

        match session {
            Some(session) => {
                let admin_repo = AdminRepository::new(state);
                let admin = admin_repo
                    .find_by_id(session.admin_id)
                    .await
                    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to find admin"))?;
                Ok(Self(Some((session, admin))))
            }
            None => Ok(Self(None)),
        }
    }
}