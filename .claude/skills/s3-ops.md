<!-- Purpose: Documents the Claude skill for S3-related development tasks. -->

---
name: s3-ops
description: Use when working on AWS S3 integration in FFM — browsing buckets, uploading/downloading files, VirtualPath parsing, and S3Client operations.
---

# S3 Operations

## Overview
FFM supports dual-panel browsing of AWS S3 alongside local filesystem. S3 integration is in `src/s3.rs`.

## Key Types
- `VirtualPath::S3 { profile, bucket, prefix }` — S3 location
- `VirtualPath::Local(PathBuf)` — local path
- `S3Client` — blocking S3 wrapper over aws-sdk-s3
- `AwsProfile` — parsed from `~/.aws/credentials`

## Path Format
```
s3://bucket/prefix          # default profile
s3://profile@bucket/prefix  # named profile
```

## Commands
```bash
# Test S3 module
cargo test s3 -q

# Test VirtualPath parsing
cargo test virtual_path -q

# Run with S3 access
AWS_PROFILE=default cargo run
```

## Notes
- S3 operations use tokio runtime (blocking bridge)
- Alt+F1/F2 shows S3 profiles alongside local drives
- Needs `~/.aws/credentials` configured
