//! Native engine discovery and link decisions shared with isolated tests.
//!
//! A build script is a host binary; nothing in it is reachable from
//! `cargo test`. The functions here are the pure path-and-name logic — the
//! part that is wrong on a machine the author does not have — so
//! The build-script test target pins these against fabricated build trees.

use super::{from_build_tree, probe, Found};
use std::path::{Path, PathBuf};

/// Search explicit, user and system installations in their documented order.
pub fn locate_in(
    explicit: Option<(PathBuf, PathBuf)>,
    home: Option<PathBuf>,
    prefixes: &[&Path],
) -> Option<Found> {
    if let Some((include, build)) = explicit {
        if let Some(found) = probe(include, build.join("src"), &build.join("src")) {
            return Some(found);
        }
    }
    if let Some(home) = home {
        let root = home.join(".local/share/ffm/Colorer-library");
        if let Some(found) = from_build_tree(&root, &root.join("_build")) {
            return Some(found);
        }
    }
    for prefix in prefixes {
        if let Some(found) = probe(
            prefix.join("include"),
            prefix.join("include"),
            &prefix.join("lib"),
        ) {
            return Some(found);
        }
    }
    None
}

/// The libraries Colorer needs, named as the target's usual packager names
/// them. Colorer uses ICU for strings and regexes, libxml2 to read HRC and
/// HRD, and zlib for packed catalogs. A Unix system install calls them
/// `icui18n`/`icudata`/`xml2`/`z`; vcpkg — what `ffm.cmd colorer` builds
/// against, Windows having no system copies — calls them
/// `icuin`/`icudt`/`libxml2`/`zlib`. The wrong set fails the link with
/// LNK1181 or `cannot find -l`, so the default has to follow the target.
pub fn default_link_libs(target_os: &str) -> &'static str {
    if target_os == "windows" {
        "icuuc,icuin,icudt,libxml2,zlib"
    } else {
        "icuuc,icui18n,icudata,xml2,z"
    }
}

/// The vcpkg-installed import libraries that belong to a located engine.
///
/// `ffm.cmd colorer` configures Colorer with vcpkg on, which puts every
/// dependency's `.lib` under `_build/vcpkg_installed/<triplet>/lib` — a
/// directory the linker never searches on its own, so without it even a
/// correct `COLORER_LINK_LIBS` dies with LNK1181. The triplet directory is
/// whichever child holds a `lib/` (vcpkg's `vcpkg/` metadata child does not),
/// never a hard-coded `x64-windows`: an arm64 or custom-triplet build lays
/// out the same shape under another name. `ffm.cmd` builds `--config
/// Release`, and `<triplet>/lib` is where vcpkg keeps the release libraries.
///
/// The walk starts at the archive's own directory because MSVC's per-config
/// layout puts it two levels below `_build` (`src/Release`) and single-config
/// generators one (`src`); three ancestors bound the walk so a stray
/// `vcpkg_installed` higher up the filesystem is never picked up.
pub fn vcpkg_lib_dir(lib_dir: &Path) -> Option<PathBuf> {
    let installed = lib_dir
        .ancestors()
        .take(3)
        .map(|dir| dir.join("vcpkg_installed"))
        .find(|dir| dir.is_dir())?;
    installed
        .read_dir()
        .ok()?
        .flatten()
        .map(|entry| entry.path().join("lib"))
        .find(|lib| lib.is_dir())
}
