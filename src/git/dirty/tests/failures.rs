//! Purpose: Exhausted descriptors and a missing Git executable leave no false dirty result.
//!
//! Both faults are process-wide — the descriptor limit and `PATH` — so each
//! runs in a child process of its own, where no parallel test can see them.

use super::*;

#[test]
fn unavailable_capture_descriptors_leave_the_repository_unmarked() {
    crate::test_support::filesystem_faults::run(|| {
        use rustix::process::{getrlimit, setrlimit, Resource, Rlimit};

        let root = TempDir::new().unwrap();
        let previous = getrlimit(Resource::Nofile);
        let limit = Rlimit {
            current: Some(64),
            ..previous
        };
        setrlimit(Resource::Nofile, limit).unwrap();
        let mut held = Vec::new();
        while let Ok(file) = fs::File::open("/dev/null") {
            held.push(file);
        }
        let without_capture = query_dirty(root.path(), root.path());
        assert!(held.pop().is_some());
        let without_clone = query_dirty(root.path(), root.path());
        setrlimit(Resource::Nofile, previous).unwrap();

        assert!(!without_capture);
        assert!(!without_clone);
    });
}

#[test]
fn an_unavailable_git_program_leaves_the_repository_unmarked() {
    crate::test_support::filesystem_faults::run(|| {
        let root = TempDir::new().unwrap();
        let _path = crate::test_support::EnvVarGuard::set("PATH", root.path());

        assert!(!query_dirty(root.path(), root.path()));
    });
}
