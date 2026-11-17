//! Purpose: Copy, move and delete from F5, F6 and F8, and the files on disk after them.

use super::*;

#[when("я нажимаю F5")]
fn when_f5(world: &mut FfmWorld) -> StepResult {
    handle_copy(world.app()?).context("F5")?;
    Ok(())
}

#[when("я нажимаю F6")]
fn when_f6(world: &mut FfmWorld) -> StepResult {
    handle_move(world.app()?).context("F6")?;
    Ok(())
}

#[when("я нажимаю F8")]
fn when_f8(world: &mut FfmWorld) -> StepResult {
    handle_delete_confirm(world.app()?).context("F8")?;
    Ok(())
}

#[when("я подтверждаю удаление")]
fn when_confirm_delete(world: &mut FfmWorld) -> StepResult {
    handle_delete(world.app()?).context("удаление")?;
    Ok(())
}

#[when("я отменяю удаление")]
fn when_cancel_delete(world: &mut FfmWorld) -> StepResult {
    let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::empty());
    let consumed = process_popup_input(world.app()?, key);
    assert!(consumed, "диалог удаления должен принять Esc");
    Ok(())
}

#[when(expr = "я копирую каталог {string} в {string}")]
fn when_copy_dir(world: &mut FfmWorld, src: String, dst: String) -> StepResult {
    let base = world.temp_path()?;
    copy_dir_all(&base.join(&src), &base.join(&dst)).context("копировать каталог")?;
    Ok(())
}

#[then(expr = "{string} должен существовать в назначении")]
fn then_dst_exists(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(
        world.dst_path()?.join(&n).exists(),
        "{n} должен существовать в назначении"
    );
    Ok(())
}

#[then(expr = "{string} должен существовать в источнике")]
fn then_src_has(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(
        world.src_path()?.join(&n).exists(),
        "{n} должен существовать в источнике"
    );
    Ok(())
}

#[then(expr = "{string} не должен существовать в источнике")]
fn then_src_not(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(
        !world.src_path()?.join(&n).exists(),
        "{n} не должен существовать"
    );
    Ok(())
}

#[then("должен появиться диалог подтверждения удаления")]
fn then_del_confirm(world: &mut FfmWorld) -> StepResult {
    assert!(world.app()?.get_delete_confirm().is_some());
    Ok(())
}

#[then(expr = "{string} должен существовать во временном каталоге")]
fn then_temp_exists(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(
        world.temp_path()?.join(&n).exists(),
        "{n} должен существовать"
    );
    Ok(())
}

#[then(expr = "{string} должен быть каталогом в источнике")]
fn then_dir_in_src(world: &mut FfmWorld, n: String) -> StepResult {
    assert!(world.src_path()?.join(&n).is_dir());
    Ok(())
}
