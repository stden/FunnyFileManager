use super::*;
use tempfile::TempDir;

#[test]
fn descriptions_keep_quoted_names_and_are_captured_before_rendering() -> io::Result<()> {
    let directory = TempDir::new()?;
    fs::write(directory.path().join("my notes.txt"), "notes")?;
    fs::write(
        directory.path().join("descript.ion"),
        "\"my notes.txt\" A description\n",
    )?;
    let mut panel = Panel::new(directory.path().to_owned())?;
    set_view(&mut panel, PanelView::Descriptions)?;
    assert_eq!(panel.details.descriptions["my notes.txt"], "A description");
    fs::remove_file(directory.path().join("descript.ion"))?;
    assert_eq!(panel.details.descriptions["my notes.txt"], "A description");
    Ok(())
}

#[test]
fn every_mode_can_be_selected_in_an_isolated_directory() -> io::Result<()> {
    let directory = TempDir::new()?;
    let mut panel = Panel::new(directory.path().to_owned())?;
    for mode in PanelView::ALL {
        set_view(&mut panel, mode)?;
        assert_eq!(panel.view, mode);
    }
    Ok(())
}

#[test]
fn an_unreadable_description_file_is_an_error() -> io::Result<()> {
    let directory = TempDir::new()?;
    fs::create_dir(directory.path().join("descript.ion"))?;
    let mut panel = Panel::new(directory.path().to_owned())?;
    assert!(set_view(&mut panel, PanelView::Descriptions).is_err());
    Ok(())
}

#[cfg(unix)]
#[test]
fn a_symlink_shows_its_target_in_the_links_view() -> io::Result<()> {
    let directory = TempDir::new()?;
    fs::write(directory.path().join("real.txt"), "x")?;
    std::os::unix::fs::symlink("real.txt", directory.path().join("link.txt"))?;
    let mut panel = Panel::new(directory.path().to_owned())?;
    set_view(&mut panel, PanelView::Links)?;
    let target = &panel.details.links["link.txt"];
    assert!(target.contains('→'), "{target}");
    Ok(())
}

#[test]
fn zz_probe_load_entries_propagates_set_view_failure() -> io::Result<()> {
    let directory = TempDir::new()?;
    fs::write(directory.path().join("a.txt"), "x")?;
    let mut panel = Panel::new(directory.path().to_owned())?;
    set_view(&mut panel, PanelView::Descriptions)?;
    let before: Vec<String> = panel
        .entries
        .iter()
        .map(|e| e.name.as_str().to_owned())
        .collect();
    fs::create_dir(directory.path().join("descript.ion"))?;
    assert!(panel.load_entries().is_err());
    let after: Vec<String> = panel
        .entries
        .iter()
        .map(|e| e.name.as_str().to_owned())
        .collect();
    assert_eq!(before, after);
    assert!(!before.iter().any(|n| n == "descript.ion"));
    Ok(())
}

/// The Descriptions view finds the directory's description file by the same
/// rule as the listing, the sort and Ctrl+Z: its name in any letter case.
#[test]
fn the_descriptions_view_reads_the_description_file_in_any_case() -> io::Result<()> {
    let directory = TempDir::new()?;
    fs::write(directory.path().join("a.txt"), "a")?;
    fs::write(directory.path().join("DESCRIPT.ION"), "a.txt Shouted\n")?;
    let mut panel = Panel::new(directory.path().to_owned())?;
    for configured in ["", "descript.ion"] {
        panel.options.description_file = configured.to_owned();
        set_view(&mut panel, PanelView::Descriptions)?;
        assert_eq!(
            panel.details.descriptions["a.txt"], "Shouted",
            "{configured:?}"
        );
    }
    Ok(())
}

/// Far's File owners column names the owner, as the Owner sort already does;
/// a bare `986:979` tells the user nothing.
#[cfg(unix)]
#[test]
fn the_owners_view_names_the_user_and_group() -> io::Result<()> {
    use std::os::unix::fs::MetadataExt;

    let directory = TempDir::new()?;
    let file = directory.path().join("mine.txt");
    fs::write(&file, "x")?;
    let mut panel = Panel::new(directory.path().to_owned())?;
    set_view(&mut panel, PanelView::Owners)?;
    let metadata = fs::symlink_metadata(&file)?;
    let mut names = crate::directory::owner::Owners::default();
    let user = names
        .of(&metadata)
        .unwrap_or_else(|| metadata.uid().to_string());
    let group = names
        .group_of(&metadata)
        .unwrap_or_else(|| metadata.gid().to_string());
    assert_eq!(panel.details.owners["mine.txt"], format!("{user}:{group}"));
    Ok(())
}

/// An id with no passwd or group entry is still shown, as its number.
#[cfg(unix)]
#[test]
fn the_owners_view_falls_back_to_numbers_for_unknown_ids() -> io::Result<()> {
    const NOBODY_KNOWN: u32 = 4_242_424;
    let directory = TempDir::new()?;
    let file = directory.path().join("orphan.txt");
    fs::write(&file, "x")?;
    if std::os::unix::fs::chown(&file, Some(NOBODY_KNOWN), Some(NOBODY_KNOWN)).is_err() {
        eprintln!("skipped: only root can give a file to an unknown user");
        return Ok(());
    }
    let mut panel = Panel::new(directory.path().to_owned())?;
    set_view(&mut panel, PanelView::Owners)?;
    assert_eq!(panel.details.owners["orphan.txt"], "4242424:4242424");
    Ok(())
}

#[cfg(windows)]
#[test]
fn has_multiple_links_counts_hard_links() -> io::Result<()> {
    let dir = TempDir::new()?;
    let file = dir.path().join("one.txt");
    let two = dir.path().join("two.txt");
    let other = dir.path().join("other.txt");
    fs::write(&file, "x")?;
    fs::write(&other, "y")?;
    assert!(!windows::has_multiple_links(&file));
    assert!(!windows::are_same_hard_link(&file, &other));
    fs::hard_link(&file, &two)?;
    assert!(windows::has_multiple_links(&file));
    assert!(windows::are_same_hard_link(&file, &two));
    Ok(())
}
