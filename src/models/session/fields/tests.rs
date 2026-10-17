//! Unit tests: every checked session value takes what it should, refuses
//! what it must — with the field and the reason, never the value — and
//! `serde` refuses the same.

use super::{
    Field, HostName, InvalidField, KeyFile, Port, Problem, SessionName, UserName, MAX_HOST_CHARS,
    MAX_KEY_FILE_CHARS, MAX_NAME_CHARS, MAX_USER_CHARS,
};

fn invalid(field: Field, problem: Problem) -> InvalidField {
    InvalidField::new(field, problem)
}

mod tests_port;
mod tests_reporting;
mod tests_text;
