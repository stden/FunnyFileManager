//! Purpose: Pin each `Availability` a probe can report, the `recipe` status of
//! a changed command, and the words and tones the dialog shows for them.

use std::{fs, path::Path};

use serde_json::json;
use tempfile::TempDir;

use super::super::{availability, Availability, NotInstalled, Replacement, Resolved, Tone};
use crate::config::{InstallRecipe, LspSettings, ServerSettings};
use crate::lsp::install::{launch, probe::Probe, storage, InstallError};

/// A fake machine: an LSP cache and one `PATH` directory, both empty.
struct Machine {
    home: TempDir,
}

impl Machine {
    fn new() -> Self {
        let home = TempDir::new().expect("tempdir");
        fs::create_dir_all(home.path().join("cache")).expect("cache");
        fs::create_dir_all(home.path().join("bin")).expect("bin");
        Self { home }
    }

    fn cache(&self) -> std::path::PathBuf {
        self.home.path().join("cache")
    }

    fn bin(&self) -> std::path::PathBuf {
        self.home.path().join("bin")
    }

    fn probe(&self) -> Probe {
        let path = crate::complete::execs::join_paths([self.bin().as_path()]);
        Probe::new(Some(self.cache()), path)
    }

    /// Put a program on this machine's `PATH`.
    fn install_on_path(&self, name: &str) -> std::path::PathBuf {
        let program = self.bin().join(launch::executable(name));
        crate::test_support::write_executable(&program, b"program").expect("program");
        program
    }

    /// Fake a finished auto-install of `recipe`: its receipt and its program.
    fn install_recipe(&self, recipe: &InstallRecipe) -> std::path::PathBuf {
        let key = storage::digest(&serde_json::to_vec(recipe).expect("recipe"));
        let root = self.cache().join(key);
        fs::create_dir_all(&root).expect("root");
        let program = root.join(launch::executable("server"));
        crate::test_support::write_executable(&program, b"server").expect("server");
        let receipt = json!({
            "command": format!("{{install_dir}}/{}", launch::executable("server")),
            "args": ["-m", "pylsp"],
            "configuration": null,
        });
        fs::write(root.join("ready.json"), receipt.to_string()).expect("receipt");
        program
    }

    fn check(&self, language: &str, server: &ServerSettings) -> Availability {
        availability(language, server, self.home.path(), &self.probe())
    }
}

fn catalog(language: &str) -> ServerSettings {
    LspSettings::catalog().0[language].clone()
}

fn recipe_of(server: &ServerSettings) -> InstallRecipe {
    server.install.clone().expect("a catalog recipe")
}

#[test]
fn a_command_on_the_probes_path_is_configured() {
    let machine = Machine::new();
    let program = machine.install_on_path("pylsp");
    let python = catalog("python");
    assert_eq!(
        machine.check("python", &python),
        Availability::Configured(Resolved {
            executable: program,
            args: python.args.clone(),
        })
    );
    let nowhere = Probe::default();
    assert_eq!(
        availability("python", &python, machine.home.path(), &nowhere),
        Availability::NotInstalled(NotInstalled::NoCache),
        "the default probe knows no cache and no PATH"
    );
}

#[test]
fn a_catalog_recipe_is_pending_until_installed_and_then_cached() {
    let machine = Machine::new();
    let python = catalog("python");
    assert_eq!(
        machine.check("python", &python),
        Availability::NotInstalled(NotInstalled::AfterTrust)
    );
    let program = machine.install_recipe(&recipe_of(&python));
    let found = machine.check("python", &python);
    assert!(matches!(&found, Availability::Cached(resolved) if resolved.executable == program));
}

/// A changed command that is not installed does not run: the recipe's program
/// does, and the status must not call that `ready`.
#[test]
fn a_changed_command_that_is_not_found_is_replaced_by_the_recipe() {
    let machine = Machine::new();
    let mut python = catalog("python");
    python.command = "custom-pylsp".to_owned();
    assert_eq!(
        machine.check("python", &python),
        Availability::Replaced(Replacement::AfterTrust)
    );
    let program = machine.install_recipe(&recipe_of(&python));
    let found = machine.check("python", &python);
    assert_eq!(found.tag(), "recipe");
    assert_eq!(found.tone(), Tone::Pending);
    assert!(matches!(
        &found,
        Availability::Replaced(Replacement::Cached(resolved)) if resolved.executable == program
    ));
    machine.install_on_path("custom-pylsp");
    assert!(matches!(
        machine.check("python", &python),
        Availability::Configured(_)
    ));
}

/// An added server's own recipe is not the catalog's: what it installs is
/// not guessed, so it shows `recipe` too.
#[test]
fn an_added_servers_recipe_is_always_a_replacement() {
    let machine = Machine::new();
    let mut mine = catalog("python");
    mine.command = "mine-server".to_owned();
    assert_eq!(
        machine.check("mine", &mine),
        Availability::Replaced(Replacement::AfterTrust)
    );
}

#[test]
fn no_recipe_no_cache_and_a_corrupt_receipt_are_not_runnable() {
    let machine = Machine::new();
    let mut python = catalog("python");
    let recipe = recipe_of(&python);
    machine.install_recipe(&recipe);
    let key = storage::digest(&serde_json::to_vec(&recipe).expect("recipe"));
    fs::write(machine.cache().join(key).join("ready.json"), "corrupt").expect("corrupt");
    assert_eq!(
        machine.check("python", &python),
        Availability::Failed(InstallError::Recipe)
    );
    python.install = None;
    assert_eq!(
        machine.check("python", &python),
        Availability::NotInstalled(NotInstalled::AutoInstallOff)
    );
    let no_cache = Probe::new(None, None);
    assert_eq!(
        availability("python", &catalog("python"), Path::new("/srv/x"), &no_cache),
        Availability::NotInstalled(NotInstalled::NoCache)
    );
}

/// A cached Java server runs the `java` the probe finds; probing checks the
/// receipt and never runs a program to do it.
#[test]
fn a_cached_java_server_uses_the_probes_java_without_running_it() {
    let machine = Machine::new();
    let java = machine.install_on_path("java");
    let server = catalog("java");
    machine.install_recipe(&recipe_of(&server));
    let found = machine.check("java", &server);
    assert!(
        matches!(&found, Availability::Cached(resolved) if resolved.executable == java),
        "{found:?}"
    );
}

fn assert_shown(availability: &Availability, label: &str, tone: Tone, tag: &str) {
    assert_eq!(availability.label(), label);
    assert_eq!(availability.tone(), tone, "{label}");
    assert_eq!(availability.tag(), tag, "{label}");
}

#[test]
fn what_runs_is_good_and_ready_with_its_old_wording() {
    let resolved = Resolved {
        executable: "/srv/x/server".into(),
        args: Vec::new(),
    };
    let configured = Availability::Configured(resolved.clone());
    assert_shown(
        &configured,
        "found configured executable",
        Tone::Good,
        "ready",
    );
    assert_eq!(configured.resolved(), Some(&resolved));
    let cached = Availability::Cached(resolved);
    assert_shown(&cached, "installed in FFM cache", Tone::Good, "ready");
}

#[test]
fn what_waits_for_an_install_or_runs_the_recipe_is_pending() {
    let resolved = Resolved {
        executable: "/srv/x/server".into(),
        args: Vec::new(),
    };
    let after_trust = Availability::NotInstalled(NotInstalled::AfterTrust);
    assert_shown(
        &after_trust,
        "not installed; installs after workspace trust",
        Tone::Pending,
        "auto",
    );
    assert_eq!(after_trust.resolved(), None);
    let replaced = Availability::Replaced(Replacement::Cached(resolved.clone()));
    assert_shown(
        &replaced,
        "command not found; FFM runs the auto-installed server instead",
        Tone::Pending,
        "recipe",
    );
    assert_eq!(replaced.resolved(), Some(&resolved));
    assert_shown(
        &Availability::Replaced(Replacement::AfterTrust),
        "command not found; FFM will install and run the auto-install server instead",
        Tone::Pending,
        "recipe",
    );
}

#[test]
fn what_cannot_run_is_bad_with_its_old_wording() {
    assert_shown(
        &Availability::NotInstalled(NotInstalled::AutoInstallOff),
        "not installed; auto-install disabled",
        Tone::Bad,
        "missing",
    );
    assert_shown(
        &Availability::NotInstalled(NotInstalled::NoCache),
        "no installation cache configured",
        Tone::Bad,
        "missing",
    );
    let failed = Availability::Failed(InstallError::Recipe);
    assert_shown(&failed, InstallError::Recipe.label(), Tone::Bad, "error");
    assert_eq!(failed.resolved(), None);
}
