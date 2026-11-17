//! Purpose: unit tests for package update plans, one topic per child module.

use super::*;

mod tests_discovery;
mod tests_execute;
mod tests_plan;

fn exited(code: i32, reason: Option<&str>) -> StepOutcome {
    StepOutcome {
        exit: StepExit::Code(code),
        reason: reason.map(str::to_owned),
    }
}

fn apt(elevation: Elevation) -> UpdatePlan {
    UpdatePlan {
        manager: Manager::Apt,
        executable: PathBuf::from("/usr/bin/apt-get"),
        elevation,
    }
}
