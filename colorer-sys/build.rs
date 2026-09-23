//! Finds a Colorer-library to link against, and compiles the C++ shim onto it.
//!
//! This never fails the build. Colorer is how the editor *prefers* to highlight,
//! not how it must: a machine without a C++ toolchain, or without ICU, still has
//! to be able to `cargo build` a file manager. When nothing is found, the crate
//! compiles to stubs, `Colorer::open` returns `None`, and the built-in tokenizer
//! does the work.
//!
//! Looked for in order:
//!   1. `COLORER_INCLUDE_DIR` + `COLORER_BUILD_DIR` — a build you point at.
//!   2. `~/.local/share/ffm/Colorer-library` — where `ffm.sh colorer` puts one.
//!   3. `/usr/local` and `/usr` — a system install.
//!
//! Network and `CMake` are deliberately not used here. A build script that clones
//! a C++ repository turns `cargo build` into something that needs the internet
//! and can take ten minutes; `ffm.sh colorer` does that once, on purpose.

use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

// Compiled into this script and, by the same `#[path]` trick, into
// `tests/build_logic_test.rs`: the link decisions that depend only on paths
// and names are the ones a test can catch without a Windows linker.
#[path = "build_logic.rs"]
mod build_logic;

/// A Colorer-library that can actually be linked: headers, the generated
/// `Features.h`, and the archive itself.
struct Found {
    include: PathBuf,
    generated: PathBuf,
    lib_dir: PathBuf,
}

/// The archive names a Colorer build can leave behind: GNU/Clang static and
/// shared, and MSVC's, which drops the `lib` prefix.
const ARCHIVES: [&str; 3] = ["libcolorer.a", "libcolorer.so", "colorer.lib"];

/// Where the archive sits relative to the build tree's `src/`. MSVC's
/// multi-config generators add a per-configuration directory; single-config
/// ones do not.
const LIB_SUBDIRS: [&str; 3] = ["", "Release", "Debug"];

fn probe(include: PathBuf, generated: PathBuf, lib_dir: &Path) -> Option<Found> {
    let headers = include.join("colorer/ParserFactory.h").is_file()
        && generated.join("colorer/common/Features.h").is_file();
    if !headers {
        return None;
    }
    let lib_dir = LIB_SUBDIRS
        .iter()
        .map(|sub| lib_dir.join(sub))
        .find(|dir| ARCHIVES.iter().any(|name| dir.join(name).is_file()))?;
    Some(Found {
        include,
        generated,
        lib_dir,
    })
}

/// A `CMake` build tree, whose generated headers and archive sit under `src/`.
fn from_build_tree(root: &Path, build: &Path) -> Option<Found> {
    probe(root.join("src"), build.join("src"), &build.join("src"))
}

fn locate() -> Option<Found> {
    for name in ["COLORER_INCLUDE_DIR", "COLORER_BUILD_DIR"] {
        println!("cargo:rerun-if-env-changed={name}");
    }
    if let (Ok(include), Ok(build)) = (
        env::var("COLORER_INCLUDE_DIR"),
        env::var("COLORER_BUILD_DIR"),
    ) {
        let build = PathBuf::from(build);
        if let Some(found) = probe(
            PathBuf::from(&include),
            build.join("src"),
            &build.join("src"),
        ) {
            return Some(found);
        }
    }

    if let Some(found) = locate_user_install() {
        return Some(found);
    }

    // A system install: headers under include/, the archive under lib/.
    for prefix in ["/usr/local", "/usr"] {
        let prefix = Path::new(prefix);
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

fn main() {
    println!("cargo:rerun-if-changed=shim.cpp");
    println!("cargo:rustc-check-cfg=cfg(colorer)");

    // No native engine is a supported configuration: the Rust crate supplies
    // stubs and the editor falls back to its built-in tokenizer. Installation
    // remains an explicit `ffm.sh colorer` / `ffm.cmd colorer` step, so a
    // normal build should not report this expected fallback as a warning.
    let Some(found) = locate() else { return };

    cc::Build::new()
        .cpp(true)
        .std("c++20")
        .file("shim.cpp")
        .include(&found.include)
        .include(&found.generated)
        .compile("colorer_shim");

    println!("cargo:rustc-cfg=colorer");
    println!("cargo:rustc-link-search=native={}", found.lib_dir.display());
    println!("cargo:rustc-link-lib=static=colorer");

    link_dependencies(&found);
}

/// Whether the C toolchain can find `lib<name>` where it would look for it.
///
/// Asked of the compiler rather than by walking `/usr/lib` and friends: it
/// already knows its own search path, including the multiarch directories that
/// a hard-coded list gets wrong on Debian.
fn has_library(name: &str) -> bool {
    let compiler = env::var("CC").unwrap_or_else(|_| "cc".to_owned());
    ["so", "a"].iter().any(|extension| {
        let file = format!("lib{name}.{extension}");
        Command::new(&compiler)
            .arg(format!("-print-file-name={file}"))
            .output()
            .ok()
            .filter(|out| out.status.success())
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_owned())
            // The compiler echoes the bare name back when it finds nothing.
            .is_some_and(|found| found != file && Path::new(&found).exists())
    })
}

fn locate_user_install() -> Option<Found> {
    // Where `ffm.sh colorer` and `ffm.cmd colorer` install it. Windows has no
    // HOME, so USERPROFILE stands in and both scripts use the same layout.
    if let Some(home) = env::var_os("HOME").or_else(|| env::var_os("USERPROFILE")) {
        let root = PathBuf::from(home).join(".local/share/ffm/Colorer-library");
        if let Some(found) = from_build_tree(&root, &root.join("_build")) {
            return Some(found);
        }
    }

    None
}

fn link_dependencies(found: &Found) {
    // `cfg!` in a build script answers for the *host* the script was compiled
    // on; the link must follow the target, which cargo hands over in the
    // CARGO_CFG_TARGET_* variables.
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        if let Some(dir) = build_logic::vcpkg_lib_dir(&found.lib_dir) {
            println!("cargo:rustc-link-search=native={}", dir.display());
        }
    }

    // The default names follow the target's packager (see `default_link_libs`);
    // COLORER_LINK_LIBS overrides the list for any other toolchain rather than
    // forcing a patch to this file.
    println!("cargo:rerun-if-env-changed=COLORER_LINK_LIBS");
    let libs = env::var("COLORER_LINK_LIBS")
        .unwrap_or_else(|_| build_logic::default_link_libs(&target_os).to_owned());
    // `dylib`, not `static`: on Unix these are the distribution's shared
    // libraries, and vcpkg's default triplets build DLLs, so the `.lib` files
    // in the directory added above are import libraries. Only `colorer` itself
    // is the static archive the probe located. A `-static` vcpkg triplet would
    // pair static deps with the static engine, but `ffm.cmd colorer` never
    // builds one, and MSVC's linker takes the same `name.lib` either way.
    for lib in libs.split(',').map(str::trim).filter(|lib| !lib.is_empty()) {
        println!("cargo:rustc-link-lib=dylib={lib}");
    }
    // Colorer's `MemoryFile` reads schemes straight out of a `.zip` through
    // minizip, and whether those calls are in `libcolorer.a` depends on how it
    // was configured — a build against system libraries has them, an older or
    // narrower one does not. Emitted only when the system has minizip to give:
    // an unnecessary `-lminizip` is dropped again by `--as-needed`, while a
    // missing one is seven undefined `unz*` symbols at link time.
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        if has_library("minizip") {
            println!("cargo:rustc-link-lib=dylib=minizip");
        }
        // MSVC links its own C++ runtime; naming libstdc++ there is an error.
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }
}
