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
use std::path::PathBuf;

/// A throwaway copy of the tree `ffm.cmd colorer` leaves behind, removed on
/// drop so a failing assertion does not strand directories in the temp dir.
struct FakeTree {
    root: PathBuf,
}

impl FakeTree {
    fn new(name: &str, dirs: &[&str]) -> Self {
        let root = std::env::temp_dir().join(format!("ffm-{name}-{}", std::process::id()));
        for dir in dirs {
            fs::create_dir_all(root.join(dir)).expect("create fake build tree");
        }
        Self { root }
    }

    fn path(&self, dir: &str) -> PathBuf {
        self.root.join(dir)
    }
}

impl Drop for FakeTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

#[test]
fn finds_vcpkg_libs_from_the_msvc_release_layout() {
    // MSVC's multi-config generator: the archive sits in `_build/src/Release`,
    // the import libraries two directories up and under `vcpkg_installed`.
    let tree = FakeTree::new(
        "msvc-release",
        &[
            "_build/src/Release",
            "_build/vcpkg_installed/x64-windows/lib",
        ],
    );
    assert_eq!(
        vcpkg_lib_dir(&tree.path("_build/src/Release")),
        Some(tree.path("_build/vcpkg_installed/x64-windows/lib"))
    );
}

#[test]
fn triplet_is_discovered_not_hardcoded() {
    // vcpkg writes a `vcpkg/` metadata child with no `lib/` next to the real
    // triplet; the scan must skip it and take whatever triplet is present.
    let tree = FakeTree::new(
        "arm64",
        &[
            "_build/src",
            "_build/vcpkg_installed/vcpkg",
            "_build/vcpkg_installed/arm64-windows/lib",
        ],
    );
    assert_eq!(
        vcpkg_lib_dir(&tree.path("_build/src")),
        Some(tree.path("_build/vcpkg_installed/arm64-windows/lib"))
    );
}

#[test]
fn a_build_without_vcpkg_adds_no_search_path() {
    // The Linux layout, and a Windows build against manually supplied
    // libraries: no `vcpkg_installed` means no extra `-L`, not a phantom one.
    let tree = FakeTree::new("no-vcpkg", &["_build/src"]);
    assert_eq!(vcpkg_lib_dir(&tree.path("_build/src")), None);
    // A stray `vcpkg_installed` above `_build` belongs to some other build and
    // stays out of reach: the walk stops at the `_build` that owns the archive.
    let stray = FakeTree::new(
        "stray",
        &["_build/src/Release", "vcpkg_installed/x64-windows/lib"],
    );
    assert_eq!(vcpkg_lib_dir(&stray.path("_build/src/Release")), None);
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
