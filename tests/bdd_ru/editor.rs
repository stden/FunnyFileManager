//! Purpose: Editing with F4: success, a failed launch and an editor that exits with an error.

use super::*;

// Редактор (F4)
#[when("я нажимаю F4 с успешным поведением")]
fn when_f4_success(world: &mut FfmWorld) -> StepResult {
    handle_edit_test_mode(world.app()?, TestEditorBehavior::Success).context("F4 успех")?;
    Ok(())
}

#[when("я нажимаю F4 с ошибкой запуска")]
fn when_f4_fail_launch(world: &mut FfmWorld) -> StepResult {
    handle_edit_test_mode(world.app()?, TestEditorBehavior::FailLaunch)
        .context("F4 ошибка запуска")?;
    Ok(())
}

#[when("я нажимаю F4 с ошибкой выхода")]
fn when_f4_exit_error(world: &mut FfmWorld) -> StepResult {
    handle_edit_test_mode(world.app()?, TestEditorBehavior::ExitError)
        .context("F4 ошибка выхода")?;
    Ok(())
}
