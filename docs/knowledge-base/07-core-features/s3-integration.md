<!-- Purpose: Explains the S3 virtual path and bucket browsing support. -->

# ☁️ S3 Integration

FunnyFileManager supports browsing and operating on AWS S3 as if it were a local drive (via Alt+F1/F2 drive selection).

## 🛣️ VirtualPath

The central abstraction is `VirtualPath` (defined in `src/s3.rs`):

```rust
pub enum VirtualPath {
    Local(PathBuf),
    S3 {
        profile: String,
        bucket: Option<String>,
        prefix: String,
    },
}
```

Parsing supports two formats:
- `s3://bucket/prefix`
- `s3://profile@bucket/prefix`

## ☁️ S3Client

- Wraps the official AWS SDK (`aws-sdk-s3`).
- Uses a blocking Tokio runtime internally so it can be called from the synchronous TUI event loop.
- All operations are wrapped in `catch_unwind` during client creation for robustness.

Supported operations:
- List buckets
- List objects (with prefix/pagination simulation)
- Upload / Download
- Delete
- Copy within or between buckets

## ⚡ Profile Handling

- Profiles are read from `~/.aws/credentials` (standard AWS format).
- `list_aws_profiles()` returns available named profiles.
- When S3 feature is disabled at compile time, S3 drives are simply not offered.

## 🎯 Drive Selection Integration

When the user presses Alt+F1/F2:
1. `App::get_all_drives()` is called.
2. Local drives + S3 profiles (if feature enabled) are merged into `Vec<DriveEntry>`.
3. The user selects a drive → it becomes the current `VirtualPath` for that panel.

## ✏️ Important Implementation Details

- S3 paths are lazy: listing happens on navigation, not on drive selection.
- Objects that end with `/` are treated as "folders" in the UI.
- Download/upload use streaming where possible.
- Errors from S3 are surfaced via `io::Result` (mapped from AWS SDK errors).

## ✨ Feature Gating

The entire S3 module is behind the `s3` feature flag:
- `Cargo.toml`: `s3 = ["dep:aws-config", ...]`
- Many BDD tests also require both `test-helpers` and `s3`.

## 🧪 Testing

- S3 integration is primarily tested via cucumber BDD scenarios (`tests/features/` and `tests/features_ru/`).
- These tests require real or mocked AWS credentials and the `s3` feature.

## 🔒 Security / Credentials

- Never hardcode credentials.
- Relies on standard AWS credential chain (environment, shared credentials file, IAM roles, etc.).
- The MCP server exposes S3 tools for AI agents (list, upload, download, etc.).

See also:
- `src/s3.rs`
- `docs/knowledge-base/07-core-features/file-operations.md` (how operations behave on S3 paths)
- MCP documentation for AI agent S3 capabilities