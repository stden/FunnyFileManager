//! Unit tests for [`super`]: finding programs and the server, telling which
//! clients are installed, and reading every state a config can be in — all on
//! a fake machine.

use crate::test_support::FakeMachine;

use super::config_paths::join_relative;
use super::programs::find_program;
use super::*;
use crate::mcp_clients::{ClientId, ConfigFormat, ServerState};
use std::{fs, path::PathBuf};

fn machine() -> FakeMachine {
    FakeMachine::new().expect("fake machine")
}

fn folder(machine: &FakeMachine) -> ProjectDir {
    project_dir(&machine.folder()).expect("the project folder")
}

fn status(survey: &Survey, client: ClientId) -> &ClientStatus {
    survey
        .clients
        .iter()
        .find(|status| status.client == client)
        .expect("every client is surveyed")
}

fn state_of(machine: &FakeMachine, host: &Host, client: ClientId, scope: Scope) -> ConfigState {
    let survey = survey(host, scope, Some(folder(machine)));
    status(&survey, client).state.clone()
}

mod config_states;
mod detection;
mod locating;
mod targets_and_paths;
