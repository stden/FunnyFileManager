//! Purpose: What a plan runs — its description and the unattended flags.

use super::*;

/// The dialog is the last chance to see what will run; a plan that goes
/// through sudo says so.
#[test]
fn a_plan_that_needs_sudo_shows_it_in_its_description() {
    assert_eq!(
        apt(Elevation::Sudo).description(),
        "sudo apt-get update && sudo apt-get upgrade -y"
    );
    assert_eq!(
        apt(Elevation::CurrentUser).description(),
        "apt-get update && apt-get upgrade -y"
    );
}

/// The menu suspends the TUI and gives the terminal to the package manager, so
/// a command that stops to ask "Do you want to continue? [Y/n]" hangs with no
/// way to answer. Every manager that has a confirmation flag must pass it.
#[test]
fn every_upgrade_runs_unattended() {
    for manager in [Manager::Chocolatey, Manager::Apt, Manager::Brew] {
        let upgrade = manager
            .commands()
            .iter()
            .find(|args| args.contains(&"upgrade"))
            .expect("every manager upgrades something");

        // `brew` has no confirmation flag and never prompts; the other two do.
        let wants_flag = manager != Manager::Brew;
        assert_eq!(
            upgrade.contains(&"-y"),
            wants_flag,
            "{} upgrade args: {upgrade:?}",
            manager.label()
        );
    }
}
