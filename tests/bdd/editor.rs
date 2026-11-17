//! Purpose: Editing with F4: success, a failed launch and an editor that exits with an error.

use super::*;

// Editor (F4)
#[when(expr = "I press F4 with success behavior")]
fn when_f4_success(world: &mut FfmWorld) -> StepResult {
    handle_edit_test_mode(world.app()?, TestEditorBehavior::Success).context("F4 success")?;
    Ok(())
}

#[when(expr = "I press F4 with fail_launch behavior")]
fn when_f4_fail_launch(world: &mut FfmWorld) -> StepResult {
    handle_edit_test_mode(world.app()?, TestEditorBehavior::FailLaunch)
        .context("F4 fail_launch")?;
    Ok(())
}

#[when(expr = "I press F4 with exit_error behavior")]
fn when_f4_exit_error(world: &mut FfmWorld) -> StepResult {
    handle_edit_test_mode(world.app()?, TestEditorBehavior::ExitError).context("F4 exit_error")?;
    Ok(())
}
