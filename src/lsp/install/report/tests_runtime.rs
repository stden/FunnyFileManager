//! Purpose: A recipe that is not installed reads `auto` only when what it
//! installs with is on the probe's `PATH`; otherwise it is an `error` with
//! the installer's own reason.

use super::super::{Availability, NotInstalled};
use super::probed::{catalog, Machine};
use crate::lsp::install::InstallError;

/// A recipe whose installer is not on this machine cannot install: the row
/// said `auto`, "installs after workspace trust", when node, npm, Python or
/// Java was missing, and only Trust told the truth.
#[test]
fn a_recipe_whose_program_is_missing_is_an_error_before_trust() {
    let machine = Machine::new();
    let missing = |language| match machine.check(language, &catalog(language)) {
        Availability::Failed(InstallError::Missing(message)) => message,
        other => panic!("{language}: {other:?}"),
    };
    assert_eq!(
        missing("python"),
        "LSP installation requires Python 3 with venv/pip"
    );
    assert_eq!(missing("json"), "LSP installation requires Node.js and npm");
    assert_eq!(
        missing("java"),
        "Java LSP requires Java 21+ (set JAVA_HOME)"
    );
    machine.install_on_path("node");
    assert_eq!(
        missing("json"),
        "LSP installation requires npm beside Node.js"
    );
    for program in ["python3", "java"] {
        machine.install_on_path(program);
    }
    for language in ["python", "java"] {
        assert_eq!(
            machine.check(language, &catalog(language)),
            Availability::NotInstalled(NotInstalled::AfterTrust),
            "{language}"
        );
    }
}

/// A custom recipe's commands are its own business: it reads `recipe`, to
/// install after trust, whatever is on `PATH`.
#[test]
fn a_custom_recipe_is_pending_whatever_is_on_path() {
    let machine = Machine::new();
    let custom = crate::lsp::install::tests::simple_recipe();
    assert_eq!(
        machine.check("custom", &custom),
        Availability::Replaced(super::super::Replacement::AfterTrust)
    );
}
