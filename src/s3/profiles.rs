//! Purpose: AWS profile discovery from `~/.aws/credentials` and `~/.aws/config`.

/// Represents an AWS profile from ~/.aws/credentials
#[derive(Clone, Debug)]
pub struct AwsProfile {
    pub name: String,
    pub region: Option<String>,
}

/// Get list of AWS profiles from ~/.aws/credentials and merge regions from ~/.aws/config
pub fn list_aws_profiles() -> Vec<AwsProfile> {
    let (content, config) = read_aws_files();
    let mut profiles = parse_credential_profiles(&content);

    if !profiles.iter().any(|p| p.name == "default") {
        profiles.insert(
            0,
            AwsProfile {
                name: "default".to_owned(),
                region: None,
            },
        );
    }

    merge_config_regions(&mut profiles, &config);
    profiles
}

fn read_aws_files() -> (String, String) {
    let aws_dir = crate::config::user_home().map(|home| home.join(".aws"));
    let read = |name: &str| {
        aws_dir
            .as_ref()
            .and_then(|dir| crate::fileload::read_small_file(&dir.join(name), name).ok())
            .unwrap_or_default()
    };
    (read("credentials"), read("config"))
}

fn parse_credential_profiles(content: &str) -> Vec<AwsProfile> {
    let mut profiles: Vec<AwsProfile> = Vec::new();
    for line in content.lines().map(str::trim) {
        if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            profiles.push(AwsProfile {
                name: name.to_owned(),
                region: None,
            });
        } else if let Some(region) = parse_region_line(line) {
            if let Some(last) = profiles.last_mut() {
                last.region = Some(region.to_owned());
            }
        }
    }
    profiles
}

fn parse_region_line(line: &str) -> Option<&str> {
    let (k, v) = line.split_once('=')?;
    (k.trim() == "region").then(|| v.trim())
}

fn merge_config_regions(profiles: &mut [AwsProfile], config: &str) {
    let mut current_profile: Option<&str> = None;
    for line in config.lines().map(str::trim) {
        if let Some(name) = line.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            current_profile = if name == "default" {
                Some("default")
            } else {
                name.strip_prefix("profile ").map(str::trim)
            };
        } else if let (Some(prof_name), Some(region)) = (current_profile, parse_region_line(line)) {
            set_profile_region(profiles, prof_name, region);
        }
    }
}

fn set_profile_region(profiles: &mut [AwsProfile], name: &str, region: &str) {
    for p in profiles.iter_mut().filter(|p| p.name == name) {
        p.region = Some(region.to_owned());
    }
}
