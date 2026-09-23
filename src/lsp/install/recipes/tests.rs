//! Purpose: Verify recipe validation and literal custom installer arguments without network access.

use super::*;
use crate::config::InstallCommand;
use crate::lsp::install::environment::System;
use std::{collections::BTreeMap, fs};
use tempfile::TempDir;

#[test]
fn custom_recipes_expand_only_the_managed_directory_and_preserve_arguments() {
    let root = TempDir::new().unwrap();
    let steps = vec![InstallCommand {
        command: "python".into(),
        args: ["-c", "import os,sys; from pathlib import Path; Path('args').write_text(sys.argv[1]+'|'+os.environ['INSTALL_TARGET'])", "literal & value"].into_iter().map(str::to_owned).collect(),
        env: BTreeMap::from([("INSTALL_TARGET".into(), "{install_dir}".into())]),
    }];
    custom(&steps, root.path(), &AtomicBool::new(false), &System).unwrap();
    assert_eq!(
        fs::read_to_string(root.path().join("args")).unwrap(),
        format!("literal & value|{}", root.path().display())
    );
}

#[test]
fn invalid_recipes_do_not_start_package_managers() {
    let root = TempDir::new().unwrap();
    let stop = AtomicBool::new(false);
    assert_eq!(
        custom(&[], root.path(), &stop, &System),
        Err(InstallError::Recipe)
    );
    assert_eq!(
        npm(&[], root.path(), &stop, &System),
        Err(InstallError::Recipe)
    );
    for argument in ["", "--help", "bad\nname", "bad\0name"] {
        assert_eq!(package_argument(argument), Err(InstallError::Recipe));
    }
    assert!(package_argument("python-lsp-server[pyflakes]").is_ok());
    assert!(npm_cli(&root.path().join("bin/node")).is_err());
}
