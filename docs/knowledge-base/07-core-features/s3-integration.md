<!-- Purpose: Explains the S3 virtual path and what the panels do not browse yet. -->

# S3 integration

The user-facing pages are [AWS profiles](../../tutorials/aws.md) and
[S3](../../tutorials/s3.md). Local panel behavior is
[filesystems](../../tutorials/filesystems.md). This page is the module map.

## What is implemented

`src/s3.rs` (feature `s3`) owns `S3Client`: a blocking Tokio runtime around
the AWS SDK. It lists buckets, lists one delimiter page of objects, downloads,
uploads, deletes, and copies. `VirtualPath` in `src/s3/virtual_path.rs` parses
`s3://bucket/prefix` and `s3://profile@bucket/prefix`.

`App::get_all_drives` adds one `DriveEntry::S3Profile` per section in the
shared credentials file when the binary is built with `s3`. Selecting that
row does not change the panel. `select_drive` returns
`Status::S3ProfileSelected`, whose text says the browser is not available yet.

The MCP server in `mcp/src/tools/s3.rs` and `mcp/src/tools/s3_transfer.rs`
performs the same S3 calls. Its local paths stay inside the configured root.
Its tools are `s3_list_profiles`, `s3_list_buckets`, `s3_list_objects`,
`s3_download`, `s3_upload`, `s3_delete`, and `s3_copy`.

## What is not implemented

Panels store a local `PathBuf`. They never call `S3Client`. There is no
prefix delete and no S3 move. `list_objects` does follow every
`ListObjectsV2` page and treats a prefix without a trailing slash as a folder.
Credentials are the named profile in the shared credentials file, not the
full SDK default chain. The region falls back to `us-east-1`.

## Feature gate

```toml
s3 = ["dep:aws-config", "dep:aws-sdk-s3", "dep:tokio"]
```

`s3` is not a default feature. The BDD targets that need it declare
`required-features = ["test-helpers", "s3"]`.
