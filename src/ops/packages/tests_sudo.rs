//! Purpose: A system package manager runs behind `sudo --`, so its path can
//! never be read as an option of sudo's own.

use super::command;
use crate::packages::{Elevation, Manager, UpdatePlan};

#[test]
fn system_package_commands_request_sudo_with_a_literal_executable() {
    let plan = UpdatePlan {
        manager: Manager::Apt,
        executable: "/usr/bin/apt-get".into(),
        elevation: Elevation::Sudo,
    };

    let prepared = command(&plan, &plan.executable, &["update"]);

    assert_eq!(prepared.get_program(), "sudo");
    let arguments: Vec<_> = prepared.get_args().collect();
    assert_eq!(
        arguments,
        [
            std::ffi::OsStr::new("--"),
            plan.executable.as_os_str(),
            std::ffi::OsStr::new("update"),
        ]
    );
}
