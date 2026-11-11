//! Loopback HTTP mock tests for S3 bucket listing, pagination, and object transfers.

use super::{server_at, tool_json, write_file_at, Parameters, TempDir};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

static S3_ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

fn p<T: DeserializeOwned>(val: Value) -> Parameters<T> {
    Parameters(serde_json::from_value(val).expect("params"))
}

async fn read_req(stream: &mut tokio::net::TcpStream) {
    let (mut buf, mut chunk) = (Vec::new(), [0u8; 4096]);
    while !buf.windows(4).any(|w| w == b"\r\n\r\n") {
        let Ok(n) = stream.read(&mut chunk).await else {
            return;
        };
        if n == 0 {
            return;
        }
        buf.extend_from_slice(&chunk[..n]);
    }
}

async fn spawn_s3(replies: Vec<(u16, Vec<u8>)>) -> (u16, tokio::task::JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let port = listener.local_addr().expect("addr").port();
    let task = tokio::spawn(async move {
        for (status, body) in replies {
            let Ok((mut stream, _)) = listener.accept().await else {
                return;
            };
            read_req(&mut stream).await;
            let head = format!(
                "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            drop(stream.write_all(head.as_bytes()).await);
            drop(stream.write_all(&body).await);
            drop(stream.shutdown().await);
        }
    });
    (port, task)
}

fn configure_profile(dir: &Path, profile: &str, port: u16) {
    let (cfg, creds) = (dir.join("aws_cfg"), dir.join("aws_creds"));
    write_file_at(
        &cfg,
        &format!(
            "[profile {profile}]\nregion = us-east-1\nendpoint_url = http://127.0.0.1:{port}\n"
        ),
    );
    write_file_at(
        &creds,
        &format!("[{profile}]\naws_access_key_id = AKIATEST\naws_secret_access_key = secret\n"),
    );
    std::env::set_var("AWS_CONFIG_FILE", &cfg);
    std::env::set_var("AWS_SHARED_CREDENTIALS_FILE", &creds);
}

#[tokio::test]
async fn list_buckets_covers_success_and_server_error() {
    let _guard = S3_ENV_LOCK.lock().await;
    let dir = TempDir::new().expect("temp");
    let (server, _) = server_at(&dir);
    let b_xml = br#"<ListAllMyBucketsResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><Buckets><Bucket><Name>alpha</Name><CreationDate>2026-01-01T00:00:00Z</CreationDate></Bucket></Buckets></ListAllMyBucketsResult>"#.to_vec();
    let (port, task) = spawn_s3(vec![(200, b_xml), (500, b"<Error/>".to_vec())]).await;
    configure_profile(dir.path(), "mockb", port);
    let buckets = server
        .s3_list_buckets(p(json!({"profile": "mockb"})))
        .await
        .unwrap();
    assert_eq!(tool_json(&buckets)["count"], 1);
    let failed = server
        .s3_list_buckets(p(json!({"profile": "mockb"})))
        .await
        .is_err();
    drop(server);
    task.await.expect("join");
    assert!(failed);
}

#[tokio::test]
async fn list_objects_paginates_folders_and_files() {
    let _guard = S3_ENV_LOCK.lock().await;
    let dir = TempDir::new().expect("temp");
    let (server, _) = server_at(&dir);
    let p1 = br#"<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><IsTruncated>true</IsTruncated><NextContinuationToken>t2</NextContinuationToken><CommonPrefixes><Prefix>logs/2026/</Prefix></CommonPrefixes></ListBucketResult>"#.to_vec();
    let p2 = br#"<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><IsTruncated>true</IsTruncated><NextContinuationToken>t2</NextContinuationToken><Contents><Key>logs/a.txt</Key><Size>12</Size><LastModified>2026-01-02T00:00:00Z</LastModified></Contents></ListBucketResult>"#.to_vec();
    let (port, task) = spawn_s3(vec![(200, p1), (200, p2)]).await;
    configure_profile(dir.path(), "mocko", port);
    let req = json!({"profile": "mocko", "bucket": "alpha", "prefix": "logs"});
    let objs = server.s3_list_objects(p(req)).await.unwrap();
    drop(server);
    task.await.expect("join");
    assert_eq!(tool_json(&objs)["count"], 2);
}

#[tokio::test]
async fn list_level_enforces_page_limit_and_handles_trailing_slash() {
    let _guard = S3_ENV_LOCK.lock().await;
    let dir = TempDir::new().expect("temp");
    let replies = (0..100)
        .map(|i| {
            let xml = format!(
                r#"<ListBucketResult xmlns="http://s3.amazonaws.com/doc/2006-03-01/"><IsTruncated>true</IsTruncated><NextContinuationToken>tok{i}</NextContinuationToken></ListBucketResult>"#
            );
            (200, xml.into_bytes())
        })
        .collect();
    let (port, task) = spawn_s3(replies).await;
    configure_profile(dir.path(), "mockcap", port);
    let client = crate::tools::s3::make_s3_client("mockcap")
        .await
        .expect("client");
    let err = crate::s3_listing::list_level(&client, "alpha", "logs/")
        .await
        .err()
        .expect("100 page limit");
    task.await.expect("join");
    assert!(err.contains("exceeded 100 pages"), "{err}");
}

#[tokio::test]
async fn s3_download_creates_new_file_and_overwrites_existing_target() {
    let _guard = S3_ENV_LOCK.lock().await;
    let dir = TempDir::new().expect("temp");
    let (server, root) = server_at(&dir);
    let replies = vec![
        (200, b"first".to_vec()),
        (200, b"second".to_vec()),
        (404, b"<Error/>".to_vec()),
    ];
    let (port, task) = spawn_s3(replies).await;
    configure_profile(dir.path(), "mockdl", port);
    let dst = root.join("nested/dl.txt").display().to_string();
    let args = json!({"profile": "mockdl", "bucket": "b", "key": "k.txt", "local_path": dst});
    let first = server.s3_download(p(args.clone())).await.unwrap();
    assert_eq!(tool_json(&first)["size"], 5);
    let second = server.s3_download(p(args.clone())).await.unwrap();
    assert_eq!(tool_json(&second)["size"], 6);
    assert_eq!(
        std::fs::read(root.join("nested/dl.txt")).unwrap(),
        b"second"
    );
    let failed = server.s3_download(p(args)).await.is_err();
    drop(server);
    task.await.expect("join");
    assert!(failed);
}

#[tokio::test]
async fn s3_upload_delete_and_copy_execute_against_local_endpoint() {
    let _guard = S3_ENV_LOCK.lock().await;
    let dir = TempDir::new().expect("temp");
    let (server, root) = server_at(&dir);
    let src = root.join("up.txt");
    write_file_at(&src, "upload-payload");
    let cp_xml = br#"<CopyObjectResult><ETag>"abc"</ETag></CopyObjectResult>"#.to_vec();
    let (port, task) = spawn_s3(vec![(200, Vec::new()), (200, Vec::new()), (200, cp_xml)]).await;
    configure_profile(dir.path(), "mockmut", port);
    let up = json!({"profile": "mockmut", "bucket": "b", "key": "up.txt", "local_path": src.display().to_string()});
    let uploaded = server.s3_upload(p(up)).await.unwrap();
    assert_eq!(tool_json(&uploaded)["size"], 14);
    let deleted = server
        .s3_delete(p(
            json!({"profile": "mockmut", "bucket": "b", "key": "up.txt"}),
        ))
        .await
        .unwrap();
    assert_eq!(tool_json(&deleted)["deleted"], "up.txt");
    let cp = json!({"profile": "mockmut", "source_bucket": "b", "source_key": "up.txt", "dest_bucket": "b2", "dest_key": "cp.txt"});
    let copied = server.s3_copy(p(cp)).await.unwrap();
    drop(server);
    task.await.expect("join");
    assert_eq!(tool_json(&copied)["destination"], "s3://b2/cp.txt");
}
