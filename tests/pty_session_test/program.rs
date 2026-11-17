//! Purpose: A panel program keeps its output visible until an actual key press.

use super::*;

fn with_program() -> std::io::Result<Ffm> {
    use std::os::unix::fs::PermissionsExt;
    let sandbox = TempDir::new()?;
    let work = sandbox.path().join("work");
    fs::create_dir(&work)?;
    let program = work.join("demo.sh");
    fs::write(
        &program,
        "#!/bin/sh\nprintf 'program-completed\\n'\nexit 7\n",
    )?;
    fs::set_permissions(&program, fs::Permissions::from_mode(0o755))?;
    let path = std::env::var_os("PATH").unwrap_or_default();
    Ok(Ffm::start_in(sandbox, &path, &[]))
}

#[test]
fn a_finished_program_waits_for_a_key_before_the_panels_resume() {
    let mut ffm = with_program().expect("program fixture");
    ffm.screen.clear();
    ffm.type_keys(b"\x1b[F\r");
    assert!(
        ffm.wait_for(PATIENCE, |ffm| {
            contains(&ffm.screen, b"Press any key to return to FFM")
        })
        .is_some(),
        "a program must pause after exit"
    );
    assert!(contains(&ffm.screen, b"program-completed"));
    assert!(contains(&ffm.screen, b"exit status: 7"));
    resume_panels(&mut ffm);
    ffm.type_keys(b":mkdir resumed\r");
    assert!(
        ffm.wait_for(PATIENCE, |ffm| ffm.work().join("resumed").is_dir())
            .is_some(),
        "the pause must consume the key before the next command"
    );
    assert!(ffm.quit().expect("normal exit").success());
}

fn resume_panels(ffm: &mut Ffm) {
    ffm.screen.clear();
    ffm.type_keys(b"\x1b[<0;2;2M");
    ffm.type_keys(b"x");
    answer_cursor_query(ffm, 0);
    assert!(
        ffm.wait_for(PATIENCE, |ffm| contains(&ffm.screen, b"Quit"))
            .is_some(),
        "the key must resume the panels: {}",
        String::from_utf8_lossy(&ffm.screen)
    );
}
