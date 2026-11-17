//! Purpose: Unit tests for `create_folder` symbolic and hard-link branches.

use tempfile::TempDir;

use super::create_folder;
use crate::models::{LinkType, NameField, NamePrompt};
use crate::status::Status;

#[test]
fn create_folder_handles_symbolic_and_refuses_hard_link() {
    let dir = TempDir::new().expect("tempdir");
    let target_dir = dir.path().join("target_dir");
    std::fs::create_dir(&target_dir).expect("mkdir");
    let sym_prompt = NamePrompt::link_to(&target_dir, "linked_dir".to_owned());
    assert_eq!(sym_prompt.link(), LinkType::Symbolic);
    let sym_outcome = create_folder(dir.path(), "linked_dir", &sym_prompt);
    assert!(matches!(
        sym_outcome,
        Ok(()) | Err(Status::CommandOutput(_))
    ));

    let target_file = dir.path().join("target.txt");
    std::fs::write(&target_file, b"x").expect("write");
    let mut hard_prompt = NamePrompt::link_to(&target_file, "hard_dir".to_owned());
    hard_prompt.focus(NameField::Link);
    hard_prompt.cycle(true);
    assert_eq!(hard_prompt.link(), LinkType::Hard);
    let err = create_folder(dir.path(), "hard_dir", &hard_prompt).expect_err("hard link refused");
    assert_eq!(
        err,
        Status::CommandOutput("a folder cannot be hard-linked".to_owned())
    );
}
