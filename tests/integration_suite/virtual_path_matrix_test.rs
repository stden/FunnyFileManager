#![cfg(feature = "s3")]
//! Purpose: Checks S3 virtual path parsing across a matrix of valid forms.
//!

use funny_file_manager::s3::VirtualPath;
use std::path::PathBuf;

mod local_path;
mod profile_bucket;
mod profile_bucket_prefix;
mod profile_only;
