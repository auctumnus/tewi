use std::collections::HashMap;

use axum::{
    body::Bytes,
    extract::{
        Multipart,
        multipart::{Field, MultipartError},
    },
};
use serde::de::DeserializeOwned;

#[derive(Debug)]
pub struct ParsedMultipart<T, B> {
    pub fields: T,
    pub files: B,
}

#[derive(Debug)]
pub enum MultipartParseError {
    IteratorError,
    //KeyNameError,
    ValueError,
    AxumError,
}

async fn read_chunks_until_done<'a>(mut field: Field<'a>) -> Result<Vec<Bytes>, MultipartError> {
    let mut chunks = Vec::<Bytes>::new();
    while let Some(chunk) = field.chunk().await? {
        chunks.push(chunk);
    }
    return Ok(chunks);
}

pub async fn parse_multipart<T: DeserializeOwned, U: DeserializeOwned>(
    mut multipart: Multipart,
) -> Result<
    ParsedMultipart<HashMap<String, String>, HashMap<String, Vec<Bytes>>>,
    MultipartParseError,
> {
    let mut text_fields = HashMap::<String, String>::new();
    let mut files = HashMap::<String, Vec<Bytes>>::new();

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|_| MultipartParseError::IteratorError)?
    {
        let name = field
            .name()
            .ok_or(MultipartParseError::AxumError)?
            .to_string();

        match &field.content_type() {
            Some(content_type) => {
                let data = read_chunks_until_done(field)
                    .await
                    .map_err(|_| MultipartParseError::ValueError)?;
                files.insert(name.clone(), data);
            }
            None => {
                let data = field
                    .text()
                    .await
                    .map_err(|_| MultipartParseError::AxumError)?;

                text_fields.insert(name.clone(), data);
            }
        };
    }
    return Ok(ParsedMultipart {
        fields: text_fields,
        files: files,
    });
}
