//! Compile the build-script owner with its isolated unit tests.
#![allow(
    clippy::print_stdout,
    reason = "the tests capture the production build script's Cargo directives"
)]
include!("build.rs");
