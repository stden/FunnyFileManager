//! Unit tests: the dialog and a session agree in both directions, a blank
//! field takes its default, a bad one names itself, and the cursor walks the
//! controls that apply.

use super::{EditPurpose, SessionEdit, SessionField};
use crate::models::{
    Auth, AuthKind, Field, HostName, InvalidField, KeyFile, Origin, Port, Problem, Protocol,
    RemotePath, Session, UserName,
};

mod tests_controls;
mod tests_session;
