use std::collections::HashMap;

use axum::{
    RequestPartsExt,
    extract::{FromRequestParts, Path},
    http::{StatusCode, request::Parts},
};

use crate::models::boards::{Board, BoardRepository};

use super::AppState;

pub struct BoardInfo(pub Option<Board>, pub Vec<String>);

impl BoardInfo {
    #[allow(dead_code)]
    pub fn board(&self) -> Option<&Board> {
        self.0.as_ref()
    }
    #[allow(dead_code)]
    pub fn board_slugs(&self) -> &Vec<String> {
        self.1.as_ref()
    }
}

impl FromRequestParts<AppState> for BoardInfo {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let board_repo = BoardRepository::new(state);

        let board_slugs = board_repo
            .list_all_slugs()
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Can't get board data"))?;

        let path_params = parts
            .extract::<Path<HashMap<String, String>>>()
            .await
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Can't parse url"))?;

        let board = match path_params.get("slug") {
            Some(token) => {
                let raw_board = board_repo
                    .find_by_slug(&token)
                    .await
                    .map_err(|_| (StatusCode::NOT_FOUND, "Board not found"))?;
                let board = board_repo
                    .materialize(raw_board)
                    .await
                    .map_err(|_| (StatusCode::NOT_FOUND, "Board not found"))?;
                Some(board)
            }
            None => None,
        };

        return Ok(BoardInfo(board, board_slugs));
    }
}
