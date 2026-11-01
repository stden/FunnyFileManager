#![cfg(feature = "s3")]
//! Purpose: Checks S3 virtual path parsing across a matrix of valid forms.
//!

use funny_file_manager::s3::VirtualPath;
use std::path::PathBuf;

#[path = "virtual_path_matrix_test/local_path.rs"]
mod local_path;
#[path = "virtual_path_matrix_test/profile_bucket.rs"]
mod profile_bucket;
#[path = "virtual_path_matrix_test/profile_bucket_prefix.rs"]
mod profile_bucket_prefix;
#[path = "virtual_path_matrix_test/profile_only.rs"]
mod profile_only;
