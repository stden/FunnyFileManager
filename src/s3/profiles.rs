//! Purpose: AWS profile discovery from `~/.aws/credentials`.

/// Represents an AWS profile from ~/.aws/credentials
#[derive(Clone, Debug)]
pub struct AwsProfile {
    pub name: String,
    pub region: Option<String>,
}

/// Get list of AWS profiles from ~/.aws/credentials
pub fn list_aws_profiles() -> Vec<AwsProfile> {
    let content = crate::config::user_home()
        .and_then(|home| {
            crate::fileload::read_small_file(&home.join(".aws").join("credentials"), "credentials")
                .ok()
        })
        .unwrap_or_default();
    let mut profiles: Vec<_> = content
        .lines()
        .filter_map(|line| {
            let name = line.trim().strip_prefix('[')?.strip_suffix(']')?;
            Some(AwsProfile {
                name: name.to_owned(),
                region: None,
            })
        })
        .collect();

    // Always include "default" if not already present
    if !profiles.iter().any(|p| p.name == "default") {
        profiles.insert(
            0,
            AwsProfile {
                name: "default".to_owned(),
                region: None,
            },
        );
    }

    profiles
}
