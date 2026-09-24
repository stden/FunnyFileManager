//! Purpose: Tests for the WSL adapter, one behaviour per child module; the
//! shared fixtures build captured output.

use super::process::{Captured, Exit, StreamOutput};
use super::*;
use crate::models::DistributionName;
use std::time::Duration;

fn success(stdout: impl Into<Vec<u8>>) -> Captured {
    Captured {
        exit: Exit::Success,
        stdout: StreamOutput {
            bytes: stdout.into(),
            truncated: false,
        },
        stderr: StreamOutput::default(),
    }
}

fn distribution(name: &str) -> DistributionName {
    DistributionName::new(name).expect("valid distribution fixture")
}

fn utf16le(text: &str) -> Vec<u8> {
    text.encode_utf16().flat_map(u16::to_le_bytes).collect()
}

mod backend;
mod decoding;
mod outcomes;
mod refresh;
mod updates;
