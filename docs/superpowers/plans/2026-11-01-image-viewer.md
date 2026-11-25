# Image viewing and panel usability implementation plan

**Goal:** Add responsive image viewing to F3 and Ctrl+Q and improve the
discoverability of panel actions, filters, and empty states.

**Architecture:** Share bounded image decoding and terminal rendering between
the viewer and quick view. Keep decoding outside the event loop and discard
results for selections that are no longer current. Integrate panel changes
independently, preserving existing key and mouse hit regions.

**Tech stack:** Rust, Ratatui, Crossterm, terminal image protocols, Cargo tests.

## 1. Fullscreen image viewing

- [ ] Add failing tests for PNG/JPEG detection, bounded decoding, invalid and
      unreadable files, image metadata, resizing, and viewer controls.
- [ ] Implement the image model and shared renderer; support the graphics
      protocols available in VS Code Terminal and a readable fallback.
- [ ] Connect the image viewer to F3 without changing text-file behavior.
- [ ] Run the new tests and verify correct output and graphics cleanup in a PTY.

## 2. Quick-view images

- [ ] Add failing tests for a selected image, loading, selection changes,
      stale results, decoding errors, and closing the preview.
- [ ] Connect Ctrl+Q to the shared image model using bounded background work.
- [ ] Run the quick-view tests, including rapid cursor changes and resize.

## 3. Panel usability

- [ ] Add failing tests for quick-view help, filter labels, empty folders and
      empty filtered results, and narrow function-key labels and click regions.
- [ ] Implement the cues while preserving existing shortcuts and Far layout.
- [ ] Implement the currently stubbed Ctrl+I filter dialog; retain masks and
      panel state on cancel or failed reload, and test repeat editing.
- [ ] Run the panel, keymap, help, and mouse tests.

## 4. Integration and validation

- [ ] Merge the three changes in an isolated worktree; review terminal image
      lifecycle, foreground responsiveness, resource bounds, and portability.
- [ ] Document controls, supported formats, fallback behavior, and the VS Code
      image setting next to the existing viewer and quick-view documentation.
- [ ] Run `./ffm.py lint` and `cargo test`, then inspect the complete results.
- [ ] Verify the integrated viewer and preview on a representative terminal;
      report any environment-dependent checks that could not be performed.
