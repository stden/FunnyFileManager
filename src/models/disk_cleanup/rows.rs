//! Purpose: The candidate rows the Disk Cleanup dialog offers, and the folder
//! each one looks in on this machine.
//!
//! A tool's folder is the one the tool itself uses: its variable when that is
//! an absolute path, else its default under the home. An empty or relative
//! variable counts as unset, as it does for every folder the program reads.

use std::path::{Path, PathBuf};

use super::{CandidateState, CleanupCandidate, CleanupKind, TempScope};

pub(super) fn build_candidates() -> Vec<CleanupCandidate> {
    let home = crate::config::user_home();
    let home = home.as_deref();
    let mut rows = vec![CleanupCandidate {
        kind: CleanupKind::Contents,
        ..candidate("Temp files", std::env::temp_dir())
    }];
    if let Some(gradle) = gradle_home(home) {
        rows.push(candidate("Gradle caches", gradle.join("caches")));
    }
    if let Some(home) = home {
        rows.push(candidate("npm cache", home.join(".npm").join("_cacache")));
    }
    rows.extend(platform_caches(home));
    rows.push(candidate("Cargo build artifacts", PathBuf::from("target")));
    rows.extend(rust_rows(home));
    rows.extend(gradle_rows(home));
    rows.extend(maven_rows(home));
    if let Some(gradle) = gradle_home(home) {
        rows.push(candidate("Gradle wrapper cache", gradle.join("wrapper")));
    }
    if let Some(home) = home {
        rows.push(CleanupCandidate {
            kind: CleanupKind::HeapDumps,
            ..candidate("JVM heap dumps", home.to_path_buf())
        });
    }
    rows
}

#[cfg(windows)]
fn platform_caches(home: Option<&Path>) -> Vec<CleanupCandidate> {
    let Some(local) = cache_dir(home) else {
        return Vec::new();
    };
    vec![
        candidate("JetBrains caches", local.join("JetBrains")),
        candidate("Docker WSL disk", local.join("Docker").join("wsl")),
        candidate("Playwright browsers", local.join("ms-playwright")),
        candidate("pip cache", local.join("pip").join("Cache")),
    ]
}

#[cfg(not(windows))]
fn platform_caches(home: Option<&Path>) -> Vec<CleanupCandidate> {
    let Some(caches) = cache_dir(home) else {
        return Vec::new();
    };
    vec![
        candidate("JetBrains caches", caches.join("JetBrains")),
        candidate("Playwright browsers", caches.join("ms-playwright")),
        candidate("pip cache", caches.join("pip")),
        candidate("uv cache", caches.join("uv")),
        candidate("Hugging Face cache", caches.join("huggingface")),
        candidate("Puppeteer browsers", caches.join("puppeteer")),
    ]
}

#[cfg(windows)]
fn cache_dir(home: Option<&Path>) -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .or_else(|| home.map(|home| home.join("AppData").join("Local")))
}

#[cfg(target_os = "macos")]
fn cache_dir(home: Option<&Path>) -> Option<PathBuf> {
    home.map(|home| home.join("Library").join("Caches"))
}

#[cfg(not(any(windows, target_os = "macos")))]
fn cache_dir(home: Option<&Path>) -> Option<PathBuf> {
    tool_dir("XDG_CACHE_HOME", home, ".cache")
}

/// The Rust toolchain's leftovers: rustup's temporary folder, which is emptied,
/// its download cache, Cargo's package-cache lock file and registry cache, and
/// the sccache cache.
fn rust_rows(home: Option<&Path>) -> Vec<CleanupCandidate> {
    let mut rows = Vec::new();
    if let Some(rustup) = tool_dir("RUSTUP_HOME", home, ".rustup") {
        rows.push(CleanupCandidate {
            kind: CleanupKind::Contents,
            ..candidate("Rustup temp files", rustup.join("tmp"))
        });
        rows.push(candidate("Rustup downloads", rustup.join("downloads")));
    }
    if let Some(cargo) = tool_dir("CARGO_HOME", home, ".cargo") {
        rows.push(candidate(
            "Cargo package-cache lock",
            cargo.join(".package-cache"),
        ));
        rows.push(candidate(
            "Cargo registry cache",
            cargo.join("registry").join("cache"),
        ));
    }
    let sccache = env_dir("SCCACHE_DIR").or_else(|| cache_dir(home).map(|c| c.join("sccache")));
    if let Some(sccache) = sccache {
        rows.push(candidate("sccache cache", sccache));
    }
    rows
}

/// Gradle's home: `GRADLE_USER_HOME`, else `~/.gradle`.
fn gradle_home(home: Option<&Path>) -> Option<PathBuf> {
    tool_dir("GRADLE_USER_HOME", home, ".gradle")
}

/// The daemon logs and the transforms cache, both under Gradle's home.
fn gradle_rows(home: Option<&Path>) -> Vec<CleanupCandidate> {
    let Some(gradle) = gradle_home(home) else {
        return Vec::new();
    };
    vec![
        CleanupCandidate {
            kind: CleanupKind::Temp(TempScope::GradleDaemonLogs),
            ..candidate("Gradle daemon logs", gradle.join("daemon"))
        },
        CleanupCandidate {
            kind: CleanupKind::Temp(TempScope::GradleTransforms),
            ..candidate("Gradle transforms", gradle.join("caches"))
        },
    ]
}

/// Maven's local repository, of which only the partial-download markers go.
fn maven_rows(home: Option<&Path>) -> Vec<CleanupCandidate> {
    let Some(home) = home else {
        return Vec::new();
    };
    vec![CleanupCandidate {
        kind: CleanupKind::Temp(TempScope::MavenPartialDownloads),
        ..candidate(
            "Maven partial downloads",
            home.join(".m2").join("repository"),
        )
    }]
}

/// `variable` when it names an absolute folder, else `default` under the home.
fn tool_dir(variable: &str, home: Option<&Path>, default: &str) -> Option<PathBuf> {
    env_dir(variable).or_else(|| home.map(|home| home.join(default)))
}

/// The folder `variable` names, when that is an absolute path.
fn env_dir(variable: &str) -> Option<PathBuf> {
    std::env::var_os(variable)
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
}

fn candidate(label: &'static str, path: PathBuf) -> CleanupCandidate {
    CleanupCandidate {
        label,
        path,
        kind: CleanupKind::Path,
        state: CandidateState::Absent,
    }
}
