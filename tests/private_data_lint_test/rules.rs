//! Purpose: The rules that recognise private data: e-mail, paths, secrets, addresses and site
//! words.

use super::*;

/// Fixture user names that tests and docs may use in `/home/<name>` paths.
const FIXTURE_USERS: &[&str] = &[
    "bob", "ann", "alice", "user", "users", "me", "you", "guest", "x",
];

/// Fixture service directories that tests and docs may use in `/srv/<name>` paths.
const FIXTURE_SRV: &[&str] = &[
    "x",
    "work",
    "app",
    "data",
    "database",
    "doc",
    "docs",
    "project",
    "project.rs",
    "root",
    "www",
    "FunnyFileManager",
];

/// E-mail domains that are, by definition, nobody's.
const PUBLIC_DOMAINS: &[&str] = &[
    "example.com",
    "example.org",
    "example.net",
    "localhost",
    "invalid",
    "test",
    "users.noreply.github.com",
];

pub struct Rules {
    email: Regex,
    home_path: Regex,
    machine_path: Regex,
    secret: Regex,
    ipv4: Regex,
    private_words: Vec<Regex>,
}

impl Rules {
    pub fn new() -> Result<Self, regex::Error> {
        Self::with_private_words(private_words())
    }

    fn with_private_words(private_words: Vec<Regex>) -> Result<Self, regex::Error> {
        Ok(Self {
            email: Regex::new(r"[A-Za-z0-9._%+-]+@([A-Za-z0-9.-]+\.[A-Za-z]{2,})")?,
            home_path: Regex::new("(/home/|/Users/|/srv/)([A-Za-z0-9_.-]+)")?,
            // Built from pieces so this file does not trip its own rule.
            machine_path: Regex::new(
                &[
                    concat!("/ro", "ot/"),
                    r"|(?i)C:\\+Users\\",
                    concat!(r"|(?i)C:\\+", "x", r"(?:\\|\b)"),
                    concat!("|/tmp/cl", "aude"),
                ]
                .concat(),
            )?,
            secret: Regex::new(concat!(
                "AKIA[0-9A-Z]{16}",
                "|gh[pousr]_[A-Za-z0-9]{30,}",
                "|github_pat_[A-Za-z0-9_]{20,}",
                r"|\bsk-[A-Za-z0-9_-]{20,}",
                "|xox[abp]-[A-Za-z0-9-]{10,}",
                "|AIza[0-9A-Za-z_-]{35}",
                "|-----BEGIN [A-Z ]*PRIVATE KEY-----",
            ))?,
            ipv4: Regex::new(r"(?:^|[^0-9.])((?:[0-9]{1,3}\.){3}[0-9]{1,3})(?:[^0-9.]|$)")?,
            private_words,
        })
    }

    pub fn check(&self, line: &str, mut report: impl FnMut(&'static str, String)) {
        for caps in self.email.captures_iter(line) {
            if !public_email(&caps[0], &caps[1]) {
                report("personal e-mail", mask(&caps[0]));
            }
        }
        for caps in self.home_path.captures_iter(line) {
            if !public_path(&caps, line) {
                report("machine-specific path", mask(&caps[0]));
            }
        }
        if let Some(found) = self.machine_path.find(line) {
            report("machine-specific path", mask(found.as_str()));
        }
        if let Some(found) = self.secret.find(line) {
            report("credential", mask(found.as_str()));
        }
        for caps in self.ipv4.captures_iter(line) {
            if !private_ip(&caps[1]) {
                report("public IP address", mask(&caps[1]));
            }
        }
        for word in &self.private_words {
            if let Some(found) = word.find(line) {
                report("private word", mask(found.as_str()));
            }
        }
    }
}

fn public_email(address: &str, domain: &str) -> bool {
    let domain = domain.to_ascii_lowercase();
    // GitHub's shared SSH login identifies a service, never a personal account.
    address == "git@github.com"
        || PUBLIC_DOMAINS
            .iter()
            .any(|d| domain == *d || domain.ends_with(&format!(".{d}")))
}

fn public_path(caps: &regex::Captures<'_>, line: &str) -> bool {
    match &caps[1] {
        "/srv/" => FIXTURE_SRV.contains(&&caps[2]),
        "/home/" if &caps[2] == "linuxbrew" => caps
            .get(0)
            .and_then(|path| line.get(path.end()..))
            .is_some_and(|suffix| suffix.starts_with("/.linuxbrew/")),
        _ => FIXTURE_USERS.contains(&&caps[2]),
    }
}

fn private_ip(ip: &str) -> bool {
    let octets: Vec<u16> = ip.split('.').filter_map(|o| o.parse().ok()).collect();
    let [a, b, ..] = octets[..] else {
        return true; // not a valid address (an octet > 255): a version number
    };
    octets.iter().any(|&o| o > 255)
        || a == 127
        || a == 10
        || a == 0
        || (a == 192 && b == 168)
        || (a == 172 && (16..=31).contains(&b))
        || (a == 169 && b == 254)
        || ip == "255.255.255.255"
        || ip == "1.2.3.4"
        || ip.starts_with("192.0.2.")
        || ip.starts_with("198.51.100.")
        || ip.starts_with("203.0.113.")
}

/// Show enough to find the line, never the whole value.
fn mask(found: &str) -> String {
    let visible: String = found.chars().take(3).collect();
    format!("{visible}…{} chars", found.chars().count())
}

fn private_words() -> Vec<Regex> {
    let path = std::env::var_os("FFM_PRIVATE_WORDS_FILE")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .or_else(|| std::env::var_os("USERPROFILE"))
                .map(|h| Path::new(&h).join(".config/ffm/private-words.txt"))
        });
    let Some(text) = path.and_then(|p| std::fs::read_to_string(p).ok()) else {
        return Vec::new();
    };
    text.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| RegexBuilder::new(l).case_insensitive(true).build().ok())
        .collect()
}

#[cfg(test)]
#[path = "rules/tests.rs"]
mod tests;
