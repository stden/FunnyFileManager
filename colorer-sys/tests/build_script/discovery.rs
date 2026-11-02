use super::super::{from_build_tree, locate_in, probe};
use super::{native_build, write};
use std::io;
use tempfile::TempDir;

#[test]
fn incomplete_builds_are_rejected() -> io::Result<()> {
    let directory = TempDir::new()?;
    let root = directory.path();
    assert!(from_build_tree(root, &root.join("_build")).is_none());
    write(&root.join("src/colorer/ParserFactory.h"))?;
    write(&root.join("_build/src/colorer/common/Features.h"))?;
    assert!(from_build_tree(root, &root.join("_build")).is_none());
    Ok(())
}

#[test]
fn supported_archives_and_configuration_directories_are_discovered() -> io::Result<()> {
    for (archive, configuration) in [
        ("libcolorer.a", ""),
        ("libcolorer.so", "Release"),
        ("colorer.lib", "Debug"),
    ] {
        let directory = TempDir::new()?;
        let root = directory.path();
        write(&root.join("include/colorer/ParserFactory.h"))?;
        write(&root.join("generated/colorer/common/Features.h"))?;
        let lib_dir = root.join("lib").join(configuration);
        write(&lib_dir.join(archive))?;
        let found = probe(
            root.join("include"),
            root.join("generated"),
            &root.join("lib"),
        )
        .expect("complete native engine");
        assert_eq!(found.lib_dir, lib_dir);
        assert_eq!(found.include, root.join("include"));
        assert_eq!(found.generated, root.join("generated"));
    }
    Ok(())
}

#[test]
fn explicit_build_precedes_user_and_system_installs() -> io::Result<()> {
    let directory = TempDir::new()?;
    let root = directory.path();
    native_build(&root.join("explicit"))?;
    native_build(&root.join("home/.local/share/ffm/Colorer-library"))?;
    let found = locate_in(
        Some((root.join("explicit/src"), root.join("explicit/_build"))),
        Some(root.join("home")),
        &[root],
    )
    .expect("explicit build");
    assert_eq!(found.lib_dir, root.join("explicit/_build/src"));
    Ok(())
}

#[test]
fn an_invalid_explicit_build_falls_back_to_the_user_install() -> io::Result<()> {
    let directory = TempDir::new()?;
    let root = directory.path();
    native_build(&root.join("home/.local/share/ffm/Colorer-library"))?;
    let found = locate_in(
        Some((root.join("missing"), root.join("missing"))),
        Some(root.join("home")),
        &[],
    )
    .expect("user build");
    assert_eq!(
        found.lib_dir,
        root.join("home/.local/share/ffm/Colorer-library/_build/src")
    );
    Ok(())
}

#[test]
fn system_prefixes_are_searched_after_missing_user_installs() -> io::Result<()> {
    let directory = TempDir::new()?;
    let root = directory.path();
    let prefix = root.join("system");
    write(&prefix.join("include/colorer/ParserFactory.h"))?;
    write(&prefix.join("include/colorer/common/Features.h"))?;
    write(&prefix.join("lib/libcolorer.a"))?;
    let found = locate_in(
        None,
        Some(root.join("missing-home")),
        &[&root.join("missing-prefix"), &prefix],
    )
    .expect("system build");
    assert_eq!(found.lib_dir, prefix.join("lib"));
    assert!(locate_in(None, None, &[]).is_none());
    Ok(())
}
