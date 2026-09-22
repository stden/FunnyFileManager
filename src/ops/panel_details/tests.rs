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
