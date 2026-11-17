# Operation and command boundaries implementation plan

> **For agentic workers:** Use `subagent-driven-development` to implement and review the independent changes below. The user approved these two refactorings after the architecture review.

**Goal:** Give name-prompt effects one owner in `ops`, and execute named automation actions through the same typed interpreters as keyboard input, without rebinding keys.

**Architecture:** Keep the current crate and modal routing. Move existing filesystem functions intact, and expose small context-specific interpreters from `popup`. Keep context validation in application automation; raw keyboard input continues through the existing resolvers and dialog protocols.

**Tech Stack:** Rust 1.95, ratatui, crossterm, existing syntax-aware architecture tests and headless integration tests.

## Task 1: Name-prompt operation ownership

Files: `src/popup/name_prompt.rs`, new `src/ops/name_prompt.rs`, `src/ops.rs`, `tests/module_boundaries/popup.rs`, `tests/module_boundaries_test.rs`.

- [x] Add a syntax-aware contract rejecting `std::fs` dependencies in production `popup::name_prompt` sources. Include a fixture proving the rule detects rename and directory creation, and a real-tree assertion.
- [x] Run `cargo test --test module_boundaries_test name_prompt`; confirm the real-tree assertion reports the existing filesystem calls.
- [x] Move `apply_name`, rename, directory creation, and post-operation refresh into `ops::name_prompt`. Keep text editing, cancellation and link-choice input in `popup::name_prompt`.
- [x] Route accepted input with `crate::ops::name_prompt::apply(app, &settings)`. Preserve no-op names, existing-target refusal, partial batch results, links and cursor placement.
- [x] Run the boundary tests and existing name-prompt/menu tests; inspect the patch for behavior changes.

## Task 2: Shared typed command interpreters

Files: `src/application/automation.rs`, remove `src/application/automation/inject.rs`, `src/popup.rs`, context owners under `src/popup/`, new `tests/module_boundaries/automation.rs`, `tests/module_boundaries_test.rs`, automation tests.

- [x] Add a syntax-aware contract rejecting keyboard-event dependencies in the named automation interpreter, with a fixture and real-tree assertion. Run `cargo test --test module_boundaries_test named_automation`; confirm the current injection bridge fails it.
- [x] Add direct interpreter tests for editor notices and nested-dialog rejection, viewer page-size parity with keyboard input, and command-line search ownership. Observe the missing typed entry points before implementing them.
- [x] Extract context-specific `apply`/`invoke` functions for viewer, editor, command-line editing/search/completion, PATH and language-server browse/view, and task search. Reuse existing WSL and vault interpreters. Preserve editor notices, nested-modal precedence and task dialog geometry transitions.
- [x] Make keyboard handlers resolve input and call those same typed functions. Keep image-specific raw keys and nested dialog input in their current owners.
- [x] Match every `Invocation` explicitly in application automation. Keep `context::validate` before dispatch and remove synthetic F24/F3 events and temporary keymap changes.
- [x] Run `cargo test --lib application::automation`, the affected popup unit tests and `cargo test --test module_boundaries_test`.

The interpreter shape is deliberately small:

```rust
pub fn apply(app: &mut App, command: ViewerCommand) {
    let page = Viewer::page_for(app.viewport);
    let Overlay::Viewer(viewer) = &mut app.overlay else {
        return;
    };
    match command {
        ViewerCommand::Close => app.close_overlay(),
        ViewerCommand::Scroll(scroll) => scroll_body(viewer, scroll, page),
    }
}
```

Keyboard-specific pre-processing stays before this call. Named invocations use the typed function after validating the active context. Owner modules stay `pub(crate)`, so these functions do not become external crate API.

## Task 3: Review and verification

- [x] Update the architecture documentation at the existing ownership and command-language explanations to record the resulting contracts.
- [x] Review spec compliance, then code quality. Check all feature configurations, especially `keepass` disabled, and the 200-line file limit.
- [x] Run `./ffm.py lint`, `cargo test`, and helper-gated automation workflows. Run artifact-dependent MCP session tests with `FFM_APPLICATION_TEST_BINARY` pointing to the built `ffm` and `-- --ignored`.
- [x] Preserve the pre-existing documentation and layer-checker changes in the working tree. Record final verification results and report before → after with KISS/DRY rules.

## Verification results

- Both new architectural contracts failed on the old owners before the refactoring and pass on the resulting code.
- Spec compliance and independent code quality reviews passed. Named vault Save/Undo in entry forms and Reveal in the password dialog have regression tests.
- `./ffm.py lint` passed, including all feature combinations and architecture, visibility and file-size rules. The full test run also passed the public API exercise contract.
- The complete `cargo test` run passed with `RUST_TEST_THREADS=1`. Tests used a temporary `python` link to the installed `python3`, and a private `TMPDIR` under `/var/tmp`: the host's pre-existing `/tmp/.git` otherwise changes fixture workspace discovery.
- Helper-gated automation and E2E targets passed: 15 and 78 tests. All seven artifact-dependent MCP session tests passed, including three BDD scenarios.
- Removing the injection source exposed a discovery bug in the duplication checker. Automatic discovery now skips removed tracked files; explicit missing paths still report errors. All nine checker tests passed.
- The additional full Python script suite still has existing failures: profiling fixtures require the absent `sx.sh`, and hook assertions expect the old architecture status label while the pre-existing layer-checker changes use the new label. These unrelated failures were left for their owning work.
