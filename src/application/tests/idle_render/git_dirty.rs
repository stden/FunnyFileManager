//! Purpose: End-to-end event-loop test verifying non-blocking load and async dirty-repo repaint.

use std::process::{Command, Stdio};

use super::*;

#[test]
fn dirty_git_repo_loads_without_blocking_and_repaints_once_in_background() -> io::Result<()> {
    let parent = TempDir::new()?;
    if !init_dirty_repo(&parent.path().join("dirty-repo"))? {
        return Ok(());
    }
    std::fs::create_dir(parent.path().join("plain-dir"))?;

    let mut app = app_at(parent.path())?;
    assert!(
        !app.left_panel.is_dirty_repo("dirty-repo"),
        "directory::read must not block synchronously on git status"
    );

    let screen = CountingBackend::new();
    let mut terminal = Terminal::new(screen.clone())?;
    let mut polls = DirtyCompletionPolls::new();
    super::super::run_loop(
        &mut terminal,
        &mut app,
        || polls.next(&screen),
        Arc::new(NoWsl),
    )?;

    assert_eq!(screen.draws(), 2);
    assert!(app.left_panel.is_dirty_repo("dirty-repo"));
    assert!(app.right_panel.is_dirty_repo("dirty-repo"));
    assert!(!app.left_panel.is_dirty_repo("plain-dir"));
    Ok(())
}

fn init_dirty_repo(repo: &std::path::Path) -> io::Result<bool> {
    let ok = Command::new("git")
        .arg("init")
        .arg(repo)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|s| s.success());
    if ok {
        std::fs::write(repo.join("uncommitted.txt"), "dirty\n")?;
        crate::git::invalidate_repo_cache_for(repo);
    }
    Ok(ok)
}

struct DirtyCompletionPolls {
    deadline: Instant,
    idle_after: usize,
}

impl DirtyCompletionPolls {
    fn new() -> Self {
        Self {
            deadline: Instant::now() + WAIT_LIMIT,
            idle_after: 0,
        }
    }

    fn next(&mut self, screen: &CountingBackend) -> io::Result<Option<Event>> {
        if screen.draws() == 1 {
            assert!(
                Instant::now() < self.deadline,
                "dirty probe did not repaint"
            );
            thread::sleep(Duration::from_millis(10));
            return Ok(None);
        }
        assert_eq!(screen.draws(), 2, "completed probe must repaint only once");
        self.idle_after += 1;
        Ok((self.idle_after == 3).then(quit))
    }
}
