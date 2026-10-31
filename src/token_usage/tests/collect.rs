//! Purpose: Missing credentials are reported independently without any network requests.
use super::*;

const CHILD: &str = "FFM_TOKEN_USAGE_MISSING_KEYS_CHILD";
const TEST: &str = "token_usage::tests::collect::missing_keys_do_not_contact_any_provider";
const DONE: &str = "fixture completed";

#[test]
fn missing_keys_do_not_contact_any_provider() {
    if std::env::var(CHILD).as_deref() == Ok("1") {
        for provider in Provider::ALL {
            assert_eq!(std::env::var(provider.key_variable()).as_deref(), Ok(""));
        }
        let results = collect();
        assert_eq!(results.len(), Provider::ALL.len());
        for (result, provider) in results.iter().zip(Provider::ALL) {
            assert_eq!(result.provider, provider);
            assert_eq!(result.result, Err(UsageError::MissingKey));
        }
        println!("{DONE}");
        return;
    }
    let mut child = std::process::Command::new(std::env::current_exe().expect("test executable"));
    child.args(["--exact", TEST, "--nocapture"]).env(CHILD, "1");
    for provider in Provider::ALL {
        child.env(provider.key_variable(), "");
    }
    let output = child.output().expect("isolated test");
    assert!(
        output.status.success(),
        "child failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains(DONE));
}
