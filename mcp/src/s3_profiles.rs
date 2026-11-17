//! Read local AWS profile names and merge optional region metadata.

use rmcp::model::CallToolResult;
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

use crate::{dirs_home, json_response};

/// List credential profiles without contacting AWS or exposing credentials.
pub fn list() -> CallToolResult {
    let profiles = dirs_home()
        .map(|home| read_profiles(&home.join(".aws")))
        .unwrap_or_default();
    json_response(&json!({ "count": profiles.len(), "profiles": profiles }))
}

/// Read profile names in credential-file order for the drive listing.
pub fn names() -> Vec<String> {
    dirs_home()
        .map(|home| read_names(&home.join(".aws")))
        .unwrap_or_default()
}

fn read_profiles(directory: &Path) -> Vec<Value> {
    let config = fs::read_to_string(directory.join("config")).unwrap_or_default();
    let mut profiles = read_names(directory)
        .into_iter()
        .map(|name| json!({ "name": name }))
        .collect::<Vec<_>>();
    merge_regions(&mut profiles, &config);
    profiles
}

fn section_name(line: &str) -> Option<&str> {
    line.trim().strip_prefix('[')?.strip_suffix(']')
}

fn read_names(directory: &Path) -> Vec<String> {
    let credentials = fs::read_to_string(directory.join("credentials")).unwrap_or_default();
    credentials
        .lines()
        .filter_map(section_name)
        .map(str::to_owned)
        .collect()
}

fn merge_regions(profiles: &mut [Value], config: &str) {
    let mut current_profile: Option<&str> = None;
    for line in config.lines().map(str::trim) {
        if let Some(name) = section_name(line) {
            current_profile = if name == "default" {
                Some("default")
            } else {
                name.strip_prefix("profile ").map(str::trim)
            };
        } else if let (Some(profile), Some((key, value))) = (current_profile, line.split_once('='))
        {
            if key.trim() == "region" {
                set_region(profiles, profile, value.trim());
            }
        }
    }
}

fn set_region(profiles: &mut [Value], name: &str, region: &str) {
    for profile in profiles {
        if profile["name"].as_str() == Some(name) {
            profile["region"] = json!(region);
        }
    }
}

#[cfg(test)]
mod tests;
