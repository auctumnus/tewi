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
pub enum MultipartParseError {
    IteratorError,
    //KeyNameError,
    ValueError,
    AxumError,
}
pub enum MultipartFormFieldErrors {
    NotText,
    NotAFile,
}

#[derive(Debug)]
pub enum MultipartFormField {
    File(Bytes),
    Text(String),
}

impl MultipartFormField {
    pub fn text(&self) -> Result<String, MultipartFormFieldErrors> {
        match self {
            MultipartFormField::Text(text) => Ok(text.to_owned()),
            _ => Err(MultipartFormFieldErrors::NotText),
        }
    }
    pub fn file(&self) -> Result<Bytes, MultipartFormFieldErrors> {
        match self {
            MultipartFormField::File(file) => Ok(file.to_owned()),
            _ => Err(MultipartFormFieldErrors::NotAFile),
        }
    }
}

async fn read_chunks_until_done<'a>(mut field: Field<'a>) -> Result<Bytes, MultipartError> {
    let mut chunks = Vec::<u8>::new();
    while let Some(chunk) = field.chunk().await? {
        chunks = [chunks, chunk.to_vec()].concat();
    }
    return Ok(Bytes::from(chunks));
}

pub async fn parse_multipart(
    mut multipart: Multipart,
) -> Result<HashMap<String, MultipartFormField>, MultipartParseError> {
    let mut fields = HashMap::<String, MultipartFormField>::new();

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
            Some(_content_type) => {
                let data = read_chunks_until_done(field)
                    .await
                    .map_err(|_| MultipartParseError::ValueError)?;
                fields.insert(name.clone(), MultipartFormField::File(data));
            }
            None => {
                let data = field
                    .text()
                    .await
                    .map_err(|_| MultipartParseError::AxumError)?;

                fields.insert(name.clone(), MultipartFormField::Text(data));
            }
        };
    }
    return Ok(fields);
}
