//! Purpose: One folder level of an S3 bucket, following every result page.

use aws_sdk_s3::operation::list_objects_v2::ListObjectsV2Output;
use aws_sdk_s3::Client;

pub(super) struct ListedObject {
    pub key: String,
    pub folder: bool,
    pub size: i64,
    pub modified: Option<String>,
}

pub(super) async fn list_level(
    client: &Client,
    bucket: &str,
    prefix: &str,
) -> Result<Vec<ListedObject>, String> {
    let prefix = folder_prefix(prefix);
    let mut objects = Vec::new();
    let mut token = None;
    loop {
        let page = request(client, bucket, &prefix, token.as_deref()).await?;
        push_page(&mut objects, &page);
        let Some(next) = continuation(&page, token.as_deref()) else {
            break;
        };
        token = Some(next);
    }
    Ok(objects)
}

fn folder_prefix(prefix: &str) -> String {
    if prefix.is_empty() || prefix.ends_with('/') {
        prefix.to_owned()
    } else {
        format!("{prefix}/")
    }
}

async fn request(
    client: &Client,
    bucket: &str,
    prefix: &str,
    token: Option<&str>,
) -> Result<ListObjectsV2Output, String> {
    let mut request = client.list_objects_v2().bucket(bucket).delimiter("/");
    if !prefix.is_empty() {
        request = request.prefix(prefix);
    }
    if let Some(token) = token {
        request = request.continuation_token(token);
    }
    request.send().await.map_err(|error| error.to_string())
}

fn push_page(objects: &mut Vec<ListedObject>, page: &ListObjectsV2Output) {
    for prefix in page.common_prefixes() {
        if let Some(key) = prefix.prefix() {
            objects.push(folder(key));
        }
    }
    for object in page.contents() {
        objects.push(file(object));
    }
}

fn folder(key: &str) -> ListedObject {
    ListedObject {
        key: key.to_owned(),
        folder: true,
        size: 0,
        modified: None,
    }
}

fn file(object: &aws_sdk_s3::types::Object) -> ListedObject {
    ListedObject {
        key: object.key().unwrap_or("").to_owned(),
        folder: false,
        size: object.size().unwrap_or(0).max(0),
        modified: object.last_modified().map(|stamp| stamp.to_string()),
    }
}

fn continuation(page: &ListObjectsV2Output, previous: Option<&str>) -> Option<String> {
    if !page.is_truncated().unwrap_or(false) {
        return None;
    }
    let token = page.next_continuation_token()?;
    if token.is_empty() || previous == Some(token) {
        return None;
    }
    Some(token.to_owned())
}
