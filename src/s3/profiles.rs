//! Purpose: AWS profile discovery from `~/.aws/credentials`.

use std::path::PathBuf;

/// Represents an AWS profile from ~/.aws/credentials
#[derive(Clone, Debug)]
pub struct AwsProfile {
    pub name: String,
    pub region: Option<String>,
}

/// Get list of AWS profiles from ~/.aws/credentials
pub fn list_aws_profiles() -> Vec<AwsProfile> {
    let content = dirs_next_candidate()
        .and_then(|home| std::fs::read_to_string(home.join(".aws").join("credentials")).ok())
        .unwrap_or_default();
    let mut profiles: Vec<_> = content
        .lines()
        .filter_map(|line| {
            let name = line.trim().strip_prefix('[')?.strip_suffix(']')?;
            Some(AwsProfile {
                name: name.to_string(),
                region: None,
            })
        })
        .collect();

    // Always include "default" if not already present
    if !profiles.iter().any(|p| p.name == "default") {
        profiles.insert(
            0,
            AwsProfile {
                name: "default".to_string(),
                region: None,
            },
        );
    }

    profiles
}

/// Get home directory candidate
fn dirs_next_candidate() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var("USERPROFILE").ok().map(PathBuf::from)
    }
    #[cfg(not(windows))]
    {
        std::env::var("HOME").ok().map(PathBuf::from)
    }
}
