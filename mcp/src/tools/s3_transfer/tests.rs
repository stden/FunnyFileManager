//! Transfers use the real SDK against isolated loopback protocol fixtures.

use super::*;
use crate::tools::s3::tests::fixture::{self, response, Fixture};
use rmcp::model::ErrorCode;

mod download;
mod mutations;
mod upload;

fn transfer(path: &str) -> Parameters<S3TransferParams> {
    Parameters(S3TransferParams {
        profile: "fixture".into(),
        bucket: "bucket".into(),
        key: "folder/object".into(),
        local_path: path.into(),
    })
}

fn copy() -> Parameters<S3CopyParams> {
    Parameters(S3CopyParams {
        profile: "fixture".into(),
        source_bucket: "source".into(),
        source_key: "folder/object".into(),
        dest_bucket: "destination".into(),
        dest_key: "copy".into(),
    })
}

fn delete() -> Parameters<S3DeleteParams> {
    Parameters(S3DeleteParams {
        profile: "fixture".into(),
        bucket: "bucket".into(),
        key: "folder/object".into(),
    })
}

fn denied() -> Vec<u8> {
    response(
        403,
        "<Error><Code>AccessDenied</Code><Message>local fixture denied</Message></Error>",
    )
}
