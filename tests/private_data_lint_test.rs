//! Purpose: keep private data out of the published repository.
//!
//! Every tracked text file is scanned for personal e-mail addresses,
//! machine-specific absolute paths, credential-shaped strings and public IP
//! addresses. Runs with the ordinary test suite, so CI refuses a commit that
//! would publish any of them.
//!
//! Site-specific words (an employer, an internal project, a host) must not be
//! spelled out here — that would publish them. Put them, one regex per line,
//! in `~/.config/ffm/private-words.txt` or the file named by
//! `FFM_PRIVATE_WORDS_FILE`; the test reads it when present and stays silent
//! when it is absent (as on CI).

use std::path::{Path, PathBuf};
use std::process::Command;

use regex::{Regex, RegexBuilder};

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

/// Binary or generated files that carry no prose.
const SKIP_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "ico", "woff", "woff2", "ttf", "pdf", "kdbx",
];

struct Finding {
    path: PathBuf,
    line: usize,
    rule: &'static str,
    hint: String,
}

#[test]
fn tracked_files_carry_no_private_data() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rules = Rules::new();
    let mut findings = Vec::new();
    for path in tracked_files(&root) {
        let Ok(text) = std::fs::read_to_string(root.join(&path)) else {
            continue; // not UTF-8: a binary we did not list
        };
        for (index, line) in text.lines().enumerate() {
            rules.check(line, |rule, hint| {
                findings.push(Finding {
                    path: path.clone(),
                    line: index + 1,
                    rule,
                    hint,
                });
            });
        }
    }
    let report: Vec<String> = findings
        .iter()
        .map(|f| format!("{}:{}: {} ({})", f.path.display(), f.line, f.rule, f.hint))
        .collect();
    assert!(
        findings.is_empty(),
        "private data must not be published — fix or allow-list:\n{}",
        report.join("\n")
    );
}

struct Rules {
    email: Regex,
    home_path: Regex,
    machine_path: Regex,
    secret: Regex,
    ipv4: Regex,
    private_words: Vec<Regex>,
}

impl Rules {
    fn new() -> Self {
        Self {
            email: Regex::new(r"[A-Za-z0-9._%+-]+@([A-Za-z0-9.-]+\.[A-Za-z]{2,})").unwrap(),
            home_path: Regex::new(r"(/home/|/Users/|/srv/)([A-Za-z0-9_.-]+)").unwrap(),
            // Built from pieces so this file does not trip its own rule.
            machine_path: Regex::new(
                &[
                    concat!("/ro", "ot/"),
                    r"|C:\\Users\\",
                    concat!(r"|C:\\", "x", r"\\"),
                    concat!("|/tmp/cl", "aude"),
                ]
                .concat(),
            )
            .unwrap(),
            secret: Regex::new(concat!(
                "AKIA[0-9A-Z]{16}",
                "|gh[pousr]_[A-Za-z0-9]{30,}",
                "|github_pat_[A-Za-z0-9_]{20,}",
                "|sk-[A-Za-z0-9_-]{20,}",
                "|xox[abp]-[A-Za-z0-9-]{10,}",
                "|AIza[0-9A-Za-z_-]{35}",
                "|-----BEGIN [A-Z ]*PRIVATE KEY-----",
            ))
            .unwrap(),
            ipv4: Regex::new(r"(?:^|[^0-9.])((?:[0-9]{1,3}\.){3}[0-9]{1,3})(?:[^0-9.]|$)").unwrap(),
            private_words: private_words(),
        }
    }

    fn check(&self, line: &str, mut report: impl FnMut(&'static str, String)) {
        for caps in self.email.captures_iter(line) {
            if !public_domain(&caps[1]) {
                report("personal e-mail", mask(&caps[0]));
            }
        }
        for caps in self.home_path.captures_iter(line) {
            let allowed = match &caps[1] {
                "/srv/" => FIXTURE_SRV.contains(&&caps[2]),
                _ => FIXTURE_USERS.contains(&&caps[2]),
            };
            if !allowed {
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

fn public_domain(domain: &str) -> bool {
    let domain = domain.to_ascii_lowercase();
    PUBLIC_DOMAINS
        .iter()
        .any(|d| domain == *d || domain.ends_with(&format!(".{d}")))
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
            std::env::var_os("HOME").map(|h| Path::new(&h).join(".config/ffm/private-words.txt"))
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

fn tracked_files(root: &Path) -> Vec<PathBuf> {
    let output = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root)
        .output()
        .expect("git must be available to list tracked files");
    assert!(output.status.success(), "git ls-files failed");
    output
        .stdout
        .split(|&b| b == 0)
        .filter(|p| !p.is_empty())
        .map(|p| PathBuf::from(String::from_utf8_lossy(p).into_owned()))
        .filter(|p| {
            !p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| SKIP_EXTENSIONS.contains(&e))
        })
        .collect()
}
