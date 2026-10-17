//! Purpose: Process subfolders: recursion, untouched inner times, links out and links round.

use super::*;

/// Process subfolders reaches what is inside; without it the folder alone
/// changes.
#[cfg(unix)]
#[test]
fn process_subfolders_recurses_and_its_absence_does_not() {
    let temp = TempDir::new().expect("temp");
    let dir = temp.path().join("d");
    let inner = dir.join("inner.txt");
    let deeper = dir.join("deep").join("x.txt");
    fs::create_dir_all(deeper.parent().expect("parent")).expect("mkdir");
    fs::write(&inner, "x").expect("write");
    fs::write(&deeper, "x").expect("write");
    set_mode(&inner, 0o644);
    set_mode(&deeper, 0o644);
    let mut app = app_at(temp.path());
    app.left_panel.select_named("d");
    open(&mut app);
    let mut dialog = dialog(&app).clone();
    set_flag(&mut dialog, Flag::OtherWrite);
    app.close_overlay();

    apply(&mut app, &dialog);
    assert_eq!(mode(&dir) & 0o002, 0o002);
    assert_eq!(mode(&inner), 0o644, "not asked to recurse");

    dialog.process_subfolders = true;
    apply(&mut app, &dialog);
    assert_eq!(mode(&inner), 0o646);
    assert_eq!(mode(&deeper), 0o646);
}

#[cfg(unix)]
fn set_mtime(path: &Path, text: &str) {
    let time = chrono::NaiveDateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S")
        .expect("fixture time")
        .and_local_timezone(chrono::Local)
        .single()
        .expect("unambiguous fixture time");
    let file = fs::File::open(path).expect("open fixture");
    file.set_modified(std::time::SystemTime::from(time))
        .expect("set fixture mtime");
}

/// Changing one bit with Process subfolders on changes that bit inside and
/// nothing else: the folder's own times, which the dialog offers back
/// untouched, are not stamped on its contents.
#[cfg(unix)]
#[test]
fn process_subfolders_leaves_the_times_inside_alone_when_they_were_not_edited() {
    let temp = TempDir::new().expect("temp");
    let dir = temp.path().join("attrtest");
    let file = dir.join("a.txt");
    let deeper = dir.join("sub").join("b.txt");
    fs::create_dir_all(deeper.parent().expect("parent")).expect("mkdir");
    fs::write(&file, "a").expect("write");
    fs::write(&deeper, "b").expect("write");
    set_mtime(&file, "2001-01-01 01:01:01");
    set_mtime(&deeper, "2002-02-02 02:02:02");
    set_mtime(&dir, "2010-10-10 10:10:10");
    let mut app = app_at(temp.path());
    app.left_panel.select_named("attrtest");
    open(&mut app);
    let mut dialog = dialog(&app).clone();
    set_flag(&mut dialog, Flag::OtherWrite);
    dialog.process_subfolders = true;
    app.close_overlay();

    apply(&mut app, &dialog);

    assert_eq!(mode(&file) & 0o002, 0o002, "the bit reached inside");
    assert_eq!(mtime(&file), "2001-01-01 01:01:01");
    assert_eq!(mtime(&deeper), "2002-02-02 02:02:02");
}

/// Opens the dialog on `name`, asks for Other Write with Process subfolders
/// on, and presses Set.
#[cfg(unix)]
fn set_other_write_recursively(app: &mut App, name: &str) {
    app.left_panel.select_named(name);
    open(app);
    let mut dialog = dialog(app).clone();
    set_flag(&mut dialog, Flag::OtherWrite);
    dialog.process_subfolders = true;
    app.close_overlay();
    apply(app, &dialog);
}

/// Process subfolders walks real folders only: a link inside the selection
/// is neither followed nor changed, the rule `chmod -R` keeps.
#[cfg(unix)]
#[test]
fn process_subfolders_never_follows_a_link_out_of_the_selection() {
    let temp = TempDir::new().expect("temp");
    let dir = temp.path().join("attrtest");
    let outside = temp.path().join("attr-outside");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::create_dir_all(&outside).expect("mkdir");
    fs::write(dir.join("a.txt"), "a").expect("write");
    fs::write(outside.join("c.txt"), "c").expect("write");
    set_mode(&outside, 0o755);
    set_mode(&outside.join("c.txt"), 0o644);
    std::os::unix::fs::symlink("../attr-outside", dir.join("link-out")).expect("symlink");
    let mut app = app_at(temp.path());

    set_other_write_recursively(&mut app, "attrtest");

    assert_eq!(mode(&dir.join("a.txt")) & 0o002, 0o002, "inside changed");
    assert_eq!(mode(&outside), 0o755, "the link's target is outside");
    assert_eq!(mode(&outside.join("c.txt")), 0o644, "and so is its content");
    assert_eq!(app.status_text(), "Attributes set on 2 object(s)");
}

/// A link back to its own folder is not a way down: the walk ends instead
/// of recursing until the kernel refuses the fortieth link.
#[cfg(unix)]
#[test]
fn process_subfolders_does_not_loop_through_a_link_to_its_own_folder() {
    let temp = TempDir::new().expect("temp");
    let sub = temp.path().join("attrtest2").join("sub");
    fs::create_dir_all(&sub).expect("mkdir");
    fs::write(sub.join("x.txt"), "x").expect("write");
    std::os::unix::fs::symlink(".", sub.join("self")).expect("symlink");
    let mut app = app_at(temp.path());

    set_other_write_recursively(&mut app, "attrtest2");

    assert_eq!(app.status_text(), "Attributes set on 3 object(s)");
}
