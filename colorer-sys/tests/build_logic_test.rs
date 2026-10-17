//! The Windows link decisions of `build.rs`, pinned on a fabricated tree.
//!
//! The real proof is a `link.exe` run against vcpkg on a Windows machine,
//! which CI here does not have. What a test *can* hold still is the path and
//! name logic: which directory the build script adds to the linker's search
//! path, and which library names it emits when `COLORER_LINK_LIBS` is unset.
//! Both were wrong once — the vcpkg lib directory was never added, and the
//! Unix names were the default on every target — and either mistake is
//! LNK1181 on the first machine that actually links the engine.

#[path = "../build_logic.rs"]
mod build_logic;

use build_logic::{default_link_libs, vcpkg_lib_dir};
use std::fs;
use std::io;
use std::path::PathBuf;

/// A throwaway copy of the tree `ffm.cmd colorer` leaves behind, removed on
/// drop so a failing assertion does not strand directories in the temp dir.
struct FakeTree {
    root: PathBuf,
}

impl FakeTree {
    /// The tree owns its root before the first directory is made, so a
    /// creation error part-way through still removes what was already made.
    /// The error names the directory that failed: a test that returns it
    /// prints only the error, with no source line to say which tree it was.
    fn new(name: &str, dirs: &[&str]) -> io::Result<Self> {
        let tree = Self {
            root: Self::root_of(name),
        };
        for dir in dirs {
            let path = tree.path(dir);
            fs::create_dir_all(&path).map_err(|err| {
                io::Error::new(
                    err.kind(),
                    format!("create fake build tree {}: {err}", path.display()),
                )
            })?;
        }
        Ok(tree)
    }

    fn root_of(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("ffm-{name}-{}", std::process::id()))
    }

    fn path(&self, dir: &str) -> PathBuf {
        self.root.join(dir)
    }
}

impl Drop for FakeTree {
    fn drop(&mut self) {
        // Best effort, on purpose: a directory left in the temp dir costs
        // nothing, while a panic here — during the unwind of a failed
        // assertion — would abort the whole test run.
        drop(fs::remove_dir_all(&self.root));
    }
}

#[test]
fn a_tree_that_cannot_be_made_names_the_directory_and_leaves_nothing() {
    // Every platform refuses a NUL byte in a file name, so the second
    // directory fails after the first was already made.
    let name = "unmakeable";
    let error = FakeTree::new(name, &["made", "bad\0name"])
        .err()
        .expect("a directory name with a NUL byte is refused");
    let root = FakeTree::root_of(name);
    let failed = root.join("bad\0name");
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    assert!(
        error
            .to_string()
            .starts_with(&format!("create fake build tree {}: ", failed.display())),
        "{error}"
    );
    assert!(!root.exists());
}

#[test]
fn finds_vcpkg_libs_from_the_msvc_release_layout() -> io::Result<()> {
    // MSVC's multi-config generator: the archive sits in `_build/src/Release`,
    // the import libraries two directories up and under `vcpkg_installed`.
    let tree = FakeTree::new(
        "msvc-release",
        &[
            "_build/src/Release",
            "_build/vcpkg_installed/x64-windows/lib",
        ],
    )?;
    assert_eq!(
        vcpkg_lib_dir(&tree.path("_build/src/Release")),
        Some(tree.path("_build/vcpkg_installed/x64-windows/lib"))
    );
    Ok(())
}

#[test]
fn triplet_is_discovered_not_hardcoded() -> io::Result<()> {
    // vcpkg writes a `vcpkg/` metadata child with no `lib/` next to the real
    // triplet; the scan must skip it and take whatever triplet is present.
    let tree = FakeTree::new(
        "arm64",
        &[
            "_build/src",
            "_build/vcpkg_installed/vcpkg",
            "_build/vcpkg_installed/arm64-windows/lib",
        ],
    )?;
    assert_eq!(
        vcpkg_lib_dir(&tree.path("_build/src")),
        Some(tree.path("_build/vcpkg_installed/arm64-windows/lib"))
    );
    Ok(())
}

#[test]
fn a_build_without_vcpkg_adds_no_search_path() -> io::Result<()> {
    // The Linux layout, and a Windows build against manually supplied
    // libraries: no `vcpkg_installed` means no extra `-L`, not a phantom one.
    let tree = FakeTree::new("no-vcpkg", &["_build/src"])?;
    assert_eq!(vcpkg_lib_dir(&tree.path("_build/src")), None);
    // A stray `vcpkg_installed` above `_build` belongs to some other build and
    // stays out of reach: the walk stops at the `_build` that owns the archive.
    let stray = FakeTree::new(
        "stray",
        &["_build/src/Release", "vcpkg_installed/x64-windows/lib"],
    )?;
    assert_eq!(vcpkg_lib_dir(&stray.path("_build/src/Release")), None);
    Ok(())
}

#[test]
fn default_library_names_follow_the_target() {
    // vcpkg's names on Windows, the distributions' names everywhere else.
    // `ffm.cmd colorer` builds with vcpkg on, so these defaults must match
    // what that build installs or the link dies with LNK1181.
    assert_eq!(
        default_link_libs("windows"),
        "icuuc,icuin,icudt,libxml2,zlib"
    );
    assert_eq!(default_link_libs("linux"), "icuuc,icui18n,icudata,xml2,z");
    assert_eq!(default_link_libs("macos"), "icuuc,icui18n,icudata,xml2,z");
}
