//! Shared fixtures for the `kdbx` unit tests: real `.kdbx` files on disk, built
//! with a deliberately cheap Argon2 KDF so the suite stays fast.
//!
//! The cheap-KDF trick mirrors `src/keepass/tests.rs::cheap_db`; that helper is
//! private to its own `#[cfg(test)]` module, so it is re-stated here rather than
//! shared. Test fixtures duplicating production shapes is expected, not a DRY
//! violation.

mod sessions;
mod vault_files;

pub use sessions::{
    focus, index_of, selected_title, sorted_visible_titles, type_password, unlocked_sample,
    unlocked_sample_with_keyfile, visible_titles,
};
pub use vault_files::{vault_file, write_keyfile};

/// Master password of every fixture vault.
///
/// Deliberately made of ordinary words: `unlock_error_never_echoes_the_typed_password`
/// asserts that none of them shows up in the failure message.
pub const VAULT_PASSWORD: &str = "correct horse battery staple";

/// One entry to write into a fixture database.
pub struct Fixture {
    pub group: &'static [&'static str],
    pub title: &'static str,
    pub username: &'static str,
    pub url: &'static str,
    pub password: &'static str,
}

/// The five-entry vault most tests run against.
///
/// Chosen so each filter case is unambiguous:
/// `git` hits one title, `carol` only a user name, `192.168` only a URL,
/// `example` exactly two entries, and `Jenkins` lives two groups deep so
/// `group_path` threading is actually exercised.
///
/// Note that `keepass::list_entries` does **not** return a stable order: entries
/// within a group come back permuted differently on every run. Tests therefore
/// look positions up with [`index_of`] or pin the cursor with [`focus`] instead
/// of hard-coding indices.
pub const SAMPLE: &[Fixture] = &[
    Fixture {
        group: &[],
        title: "GitHub",
        username: "alice",
        url: "https://github.com",
        password: "gh-pw",
    },
    Fixture {
        group: &[],
        title: "Gmail",
        username: "bob@example.com",
        url: "https://mail.google.com",
        password: "gm-pw",
    },
    Fixture {
        group: &[],
        title: "Router",
        username: "admin",
        url: "192.168.1.1",
        password: "rt-pw",
    },
    Fixture {
        group: &[],
        title: "bank",
        username: "carol",
        url: "https://bank.example",
        password: "bk-pw",
    },
    Fixture {
        group: &["Work", "CI"],
        title: "Jenkins",
        username: "ci-bot",
        url: "https://ci.internal",
        password: "jk-pw",
    },
];

fn fixture(title: &str) -> &'static Fixture {
    SAMPLE
        .iter()
        .find(|entry| entry.title == title)
        .unwrap_or_else(|| panic!("no fixture titled {title}"))
}

/// The password [`SAMPLE`] gave `title`.
pub fn password_for(title: &str) -> &'static str {
    fixture(title).password
}

/// The user name [`SAMPLE`] gave `title`.
pub fn username_for(title: &str) -> &'static str {
    fixture(title).username
}
