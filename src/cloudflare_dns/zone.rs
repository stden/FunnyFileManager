//! Purpose: Which zone a hostname belongs to, and the name a record takes under that zone.

use crate::models::cloudflare_dns::Zone;

/// The zone with the longest name that ends `hostname`, on a label boundary: `example.com` ends
/// `host.example.com`, but not `hostexample.com`.
pub fn choose<'a>(zones: &'a [Zone], hostname: &str) -> Option<&'a Zone> {
    let host = normalise(hostname);
    zones
        .iter()
        .filter(|zone| covers(&host, &normalise(&zone.name)))
        .max_by_key(|zone| normalise(&zone.name).len())
}

/// The record name of `hostname` under `zone`. At the apex the name is the zone name itself.
pub fn relative_name(hostname: &str, zone: &Zone) -> String {
    let host = normalise(hostname);
    let apex = normalise(&zone.name);
    if host == apex {
        return zone.name.clone();
    }
    let suffix = format!(".{apex}");
    match host.strip_suffix(suffix.as_str()) {
        Some(relative) => relative.to_owned(),
        None => host,
    }
}

fn covers(host: &str, zone: &str) -> bool {
    host == zone
        || host
            .strip_suffix(zone)
            .is_some_and(|rest| rest.ends_with('.'))
}

/// Names compare without case and without the trailing dot of a fully qualified name.
fn normalise(name: &str) -> String {
    name.trim().trim_end_matches('.').to_ascii_lowercase()
}
