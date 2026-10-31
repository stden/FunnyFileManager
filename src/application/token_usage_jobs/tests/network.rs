//! Purpose: Network mode handles absent credentials without contacting providers.
use super::*;

const CHILD: &str = "FFM_USAGE_NETWORK_CHILD";
const NAME: &str = "application::token_usage_jobs::tests::network::network_mode_reports_missing_credentials_without_connecting";
const DONE: &str = "usage network fixture completed";

#[test]
fn network_mode_reports_missing_credentials_without_connecting() {
    if std::env::var(CHILD).as_deref() == Ok("1") {
        check_reports();
        println!("{DONE}");
        return;
    }
    let mut child = std::process::Command::new(std::env::current_exe().expect("test executable"));
    child.args(["--exact", NAME, "--nocapture"]).env(CHILD, "1");
    for provider in Provider::ALL {
        child.env(provider.key_variable(), "");
    }
    let output = child.output().expect("isolated test");
    assert!(
        output.status.success(),
        "child: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains(DONE));
}

fn check_reports() {
    for provider in Provider::ALL {
        assert_eq!(std::env::var(provider.key_variable()).as_deref(), Ok(""));
    }
    let temp = tempfile::TempDir::new().expect("temp");
    let mut app = app(&temp);
    app.live.usage = crate::models::token_usage::UsageAccess::Network;
    crate::ops::token_usage::open(&mut app);
    let mut jobs = UsageJobs::default();
    jobs.pump(&mut app);
    finish(&mut jobs, &mut app);
    let Overlay::TokenUsage(panel) = &app.overlay else {
        panic!("panel");
    };
    for provider in Provider::ALL {
        assert!(panel
            .viewer
            .lines
            .iter()
            .any(|line| line.contains(provider.label())
                && line.contains(UsageError::MissingKey.message())));
    }
}
