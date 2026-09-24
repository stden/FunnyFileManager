//! Purpose: one folder level of `ListObjectsV2`, with every truncated page followed.

use super::listing::{file_from_object, folder_from_prefix};
use super::S3Object;
use aws_sdk_s3::Client;
use std::io;

/// `photos` and `photos/` are the same folder. The bucket root stays empty.
pub(super) fn folder_prefix(prefix: &str) -> String {
    if prefix.is_empty() || prefix.ends_with('/') {
        prefix.to_owned()
    } else {
        format!("{prefix}/")
    }
}

pub(super) async fn collect_pages(
    client: &Client,
    bucket: &str,
    prefix: &str,
) -> io::Result<Vec<S3Object>> {
    let mut objects = Vec::new();
    let mut token = None;
    loop {
        let page = list_page(client, bucket, prefix, token.as_deref()).await?;
        objects.extend(rows_of(&page, prefix));
        token = continuation(&page, token.as_deref());
        if token.is_none() {
            return Ok(objects);
        }
    }
}

async fn list_page(
    client: &Client,
    bucket: &str,
    prefix: &str,
    token: Option<&str>,
) -> io::Result<aws_sdk_s3::operation::list_objects_v2::ListObjectsV2Output> {
    let mut request = client.list_objects_v2().bucket(bucket).delimiter("/");
    if !prefix.is_empty() {
        request = request.prefix(prefix);
    }
    if let Some(token) = token {
        request = request.continuation_token(token);
    }
    request
        .send()
        .await
        .map_err(|error| io::Error::other(error.to_string()))
}

fn rows_of(
    page: &aws_sdk_s3::operation::list_objects_v2::ListObjectsV2Output,
    prefix: &str,
) -> Vec<S3Object> {
    let folders = page
        .common_prefixes()
        .iter()
        .filter_map(|entry| folder_from_prefix(entry.prefix()?, prefix));
    let files = page
        .contents()
        .iter()
        .filter_map(|object| file_from_object(object, prefix));
    folders.chain(files).collect()
}

fn continuation(
    page: &aws_sdk_s3::operation::list_objects_v2::ListObjectsV2Output,
    previous: Option<&str>,
) -> Option<String> {
    if !page.is_truncated().unwrap_or(false) {
        return None;
    }
    let token = page.next_continuation_token()?;
    if token.is_empty() || previous == Some(token) {
        return None;
    }
    Some(token.to_owned())
}
