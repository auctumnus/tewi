use uuid::Uuid;

use crate::models::attachments::{attachment_path, thumbnail_path};

#[askama::filter_fn]
pub fn thumbnail_url(
    // Value that's piped into the filter within the jinja template.
    // This can be of any type. `impl Display` is just an example.
    value: Uuid,
    // This is askama's runtime values environment. Together with
    // values, these two arguments are always passed into a custom filter.
    _env: &dyn askama::Values,
) -> askama::Result<String> {
    Ok(thumbnail_path(value).to_string_lossy().to_string())

    //Ok(format!("{value} | example_filter1"))
}
#[askama::filter_fn]
pub fn attachment_url(
    // Value that's piped into the filter within the jinja template.
    // This can be of any type. `impl Display` is just an example.
    value: Uuid,
    // This is askama's runtime values environment. Together with
    // values, these two arguments are always passed into a custom filter.
    _env: &dyn askama::Values,
) -> askama::Result<String> {
    Ok(attachment_path(value).to_string_lossy().to_string())

    //Ok(format!("{value} | example_filter1"))
}
