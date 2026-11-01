<!-- Purpose: Defines measurable terminal UI quality checks and visual-review artifacts. -->

# Terminal interface quality

Run the UI quality suite locally:

```sh
cargo test --lib ui::tests::quality
```

To also export SVGs from the real terminal cells:

```sh
FFM_UI_ARTIFACT_DIR=/tmp/ffm-ui-review cargo test --lib ui::tests::quality -- --nocapture
```

In PowerShell, set `$env:FFM_UI_ARTIFACT_DIR` to a review directory before running
the same Cargo command. Artifacts contain synthetic fixture data, not the user's
files. Isolated temporary fixture paths, prompt identity and operating-system
conventions can vary between runs and platforms.

## Automatic checks

The fixture catalog exercises file panels, every panel view and menu heading,
the current Options pages, forms, warnings, help, editor, viewer, tasks, NetBox,
console, quick view, WSL statuses, LSP configuration and diagnostics, command
completion, image loading/error states and the KeePass browser when enabled.
Long Russian names, wide characters and combining accents are included. New screens belong in
`src/ui/tests/quality/fixtures.rs` and its fixture modules.

Every state is rendered at 40×12, 80×24 and 120×30 cells, using the blue, black
and gray themes, each with file highlighting enabled and disabled. The gate checks:

- Presence of essential captions and actions at the supported size.
- Dialog and button bounds, including overlapping button hit regions.
- No control characters or replacement glyphs in the rendered fixture text.
- Wide glyphs fit the screen and retain their continuation cell.
- Effective text contrast of at least 4.5:1, without rounding the result.

Contrast uses the sRGB luminance formula and ordinary-text threshold described
in [W3C's contrast guidance](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html).
This is a project readability baseline for a terminal, not a declaration of web
accessibility conformance. Bold terminal text does not receive a lower threshold.

The checker resolves foreground/background colours before applying `REVERSED`.
RGB colours are exact; ANSI names and indices 0–15 use the project's declared
reference palette. Higher indices use the
[xterm colour cube and grayscale ramp](https://www.invisible-island.net/xterm/manpage/xterm.html).
Unspecified terminal colours use white text on black. A user's terminal can
override those defaults, so owned UI colours prefer explicit RGB values.

Disabled controls, hidden-file labels and ordinary dimmed text must also meet
the text contrast threshold: a colour alone cannot exempt a label. Decorative
box drawing and image/graph block glyphs are excluded. For a modal or menu,
a separate production render identifies its
active glyph positions: their contrast is measured on the original full frame.
This excludes the inactive underlay and its shadow without exempting active
dialog text. Panels are checked separately in every theme.

The checker has negative tests for unreadable colours, missing labels, corrupt
text, wide-glyph clipping and overlapping controls. These prove that a regression
causes a failure rather than just producing another screenshot.

## Visual review

Linux and Windows CI run these checks with the normal test suite and upload
generated `ui-quality-Linux` and `ui-quality-Windows` artifacts even when tests
fail. A build failure or an early test-runner abort can prevent generation.
Each contains `report.txt`, the blue-theme SVGs at every tested size, and panel
overviews for the other theme configurations.

SVGs preserve effective colours, bold, italic, underline and strike-through.
They paint backgrounds before text so a following cell cannot cover half of a
wide glyph. Their appearance still depends on the installed monospace font;
terminal-specific dimming and blinking are not simulated. They are review
artifacts, not approved pixel baselines.

Review spacing, visual hierarchy, focus, label clarity and whether truncation
preserves meaning. Compare narrow and wide screens and both platform artifacts.
Automated checks do not judge aesthetic taste, the meaning of arbitrary user
content, or the clarity of every possible translation. Add representative states
and deliberate failure cases when changing interface text or layout.
