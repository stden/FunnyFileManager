//! Purpose: Pin the words, tones and tags the dialog shows for each
//! `Availability`, and the program each one would run.

use super::super::{Availability, NotInstalled, Replacement, Resolved, Tone};
use crate::lsp::install::InstallError;

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
