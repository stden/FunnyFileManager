//! Purpose: image viewing through the real terminal input and redraw loop.

use super::*;

// A valid one-pixel RGB PNG; keeping it in the test makes it independent of
// image-generation tools installed on the machine running the test.
const PNG: &[u8] = &[
    137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, 73, 72, 68, 82, 0, 0, 0, 1, 0, 0, 0, 1, 8, 2, 0,
    0, 0, 144, 119, 83, 222, 0, 0, 0, 12, 73, 68, 65, 84, 120, 156, 99, 248, 207, 192, 0, 0, 3, 1,
    1, 0, 201, 254, 146, 239, 0, 0, 0, 0, 73, 69, 78, 68, 174, 66, 96, 130,
];

#[expect(
    clippy::expect_used,
    reason = "test fixture: a PNG must exist before FFM starts"
)]
fn ffm_with_image(env: &[(&str, &str)]) -> Ffm {
    let sandbox = TempDir::new().expect("sandbox");
    let work = sandbox.path().join("work");
    fs::create_dir_all(&work).expect("work directory");
    fs::write(work.join("image.png"), PNG).expect("image fixture");
    let path = std::env::var_os("PATH").unwrap_or_default();
    let mut ffm = Ffm::start_with_env(sandbox, &path, &[], env);
    // The file sorts after the parent-directory row.
    ffm.type_keys(b"\x1b[F");
    ffm
}

#[test]
fn f3_opens_a_png_in_the_viewer() {
    let mut ffm = ffm_with_image(&[]);
    ffm.type_keys(b"\x1bOR");

    let shown = ffm.wait_for(PATIENCE, |ffm| contains(&ffm.screen, b"PNG"));
    assert!(shown.is_some(), "F3 never displayed image metadata");
}

#[test]
fn quick_view_shows_a_png_from_the_real_event_loop() {
    let mut ffm = ffm_with_image(&[]);
    ffm.type_keys(b"\x11"); // Ctrl+Q

    let shown = ffm.wait_for(PATIENCE, |ffm| contains(&ffm.screen, b"PNG"));
    assert!(shown.is_some(), "quick view never displayed image metadata");
}

#[test]
fn f3_sends_iterm_image_data_when_graphics_are_enabled() {
    let mut ffm = ffm_with_image(&[("FFM_IMAGE_PROTOCOL", "iterm2")]);
    ffm.type_keys(b"\x1bOR");

    let shown = ffm.wait_for(PATIENCE, |ffm| contains(&ffm.screen, b"\x1b]1337;File="));
    assert!(shown.is_some(), "F3 never sent an iTerm2 image");
}

#[test]
fn f3_image_leaves_a_terminal_row_for_its_controls() {
    let mut ffm = ffm_with_image(&[("FFM_IMAGE_PROTOCOL", "iterm2")]);
    ffm.type_keys(b"\x1bOR");
    let shown = ffm.wait_for(PATIENCE, |ffm| contains(&ffm.screen, b"\x1b]1337;File="));
    assert!(shown.is_some(), "F3 never sent an iTerm2 image");
    let output = String::from_utf8_lossy(&ffm.screen);
    let header = output
        .split("\x1b]1337;File=")
        .nth(1)
        .and_then(|rest| rest.split(':').next())
        .expect("image header");
    let height = header
        .split(';')
        .find_map(|part| part.strip_prefix("height="))
        .and_then(|height| height.parse::<u16>().ok())
        .expect("height in terminal cells");
    // The fixture terminal has 30 rows; one is the status bar and one
    // displays controls. An image reaching the bottom can scroll xterm.
    assert!(height <= 28, "image occupied {height} terminal rows");
}

#[test]
fn f3_prepares_a_new_image_when_the_terminal_is_resized() {
    let mut ffm = ffm_with_image(&[("FFM_IMAGE_PROTOCOL", "iterm2")]);
    ffm.type_keys(b"\x1bOR");
    let marker = b"\x1b]1337;File=";
    assert!(ffm
        .wait_for(PATIENCE, |ffm| contains(&ffm.screen, marker))
        .is_some());
    let previous = ffm
        .screen
        .windows(marker.len())
        .filter(|part| *part == marker)
        .count();
    let boundary = ffm.screen.len();
    ffm.resize(70, 20);
    // The test PTY is not the child's controlling terminal, so the kernel
    // does not deliver the SIGWINCH that a real terminal would send.
    ffm.signal(Signal::WINCH);
    let queried = ffm.wait_for(PATIENCE, |ffm| {
        contains(&ffm.screen[boundary..], b"\x1b[6n")
    });
    assert!(
        queried.is_some(),
        "terminal never requested the cursor position"
    );
    // A real terminal answers the cursor-position request; the bare PTY has
    // no emulator, so provide the reply ourselves.
    ffm.type_keys(b"\x1b[1;1R");

    let redrawn = ffm.wait_for(PATIENCE, |ffm| {
        ffm.screen
            .windows(marker.len())
            .filter(|part| *part == marker)
            .count()
            > previous
    });
    assert!(redrawn.is_some(), "image was not redrawn after resizing");
}
