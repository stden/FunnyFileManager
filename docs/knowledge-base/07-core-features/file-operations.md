<!-- Purpose: Documents copy, move, delete, mkdir, creation, and refresh behavior. -->

# File Operations

File-operation implementations are owned by `src/ops/file.rs`. Low-level path,
recursive-copy, and refresh helpers live in `src/utils.rs`;
`src/handlers/mod.rs` only preserves compatibility imports.

## Source Selection

Copy, move, and delete act on all marked names when the active panel has marks.
Otherwise they act on the selected entry. With neither marks nor a selection,
they return without changing the filesystem.

## Copy (F5)

- Copies from the active panel to the inactive panel directory.
- Calls `unique_destination` for every source, so an existing destination is
  not overwritten.
- Uses `copy_dir_all` for directories and `fs::copy` for files.
- Refuses to copy a directory into itself or one of its descendants.
- Removes a partially written destination when an individual copy fails.
- Continues a marked-item batch after an individual failure, reports the
  successful and failed counts, clears marks, and then refreshes both panels.

## Move (F6)

- Chooses a non-colliding destination with `unique_destination`.
- Tries `fs::rename` first.
- On any rename error, falls back to copy followed by source deletion.
- Applies the same descendant guard, partial-copy cleanup, batch error
  collection, mark clearing, and final refresh as copy.

The fallback is not fully transactional. The source is removed only after its
copy succeeds, but a successful copy is not rolled back if source deletion
fails. Earlier successful items in a multi-item operation also remain changed.

## Delete (F8)

- `handle_delete_confirm` opens `Overlay::DeleteConfirm` with the selected or
  marked paths.
- Confirmation in `popup` calls `handle_delete`.
- Files use `fs::remove_file`; directories use `fs::remove_dir_all`.
- Per-path failures are collected into the resulting status instead of stopping
  the remaining deletions.
- Marks are cleared after the attempts, then both panels are refreshed. A
  refresh failure is propagated.

## Mkdir (F7)

- Creates `new_folder`, then `new_folder_N` up to the bounded search limit.
- Uses `fs::create_dir_all`.
- Refreshes both panels and propagates refresh errors.

## New File (Shift+F4)

- `Overlay::NewFileInput` collects the name; `popup` delegates creation to
  `ops::handle_create_file`.
- Empty or whitespace-only input is rejected through status.
- An existing path is reported and never overwritten.
- On successful creation, a panel refresh is attempted. The current
  implementation deliberately does not propagate that refresh error.

## Utilities

- `unique_destination(base, filename)` — adds a `_N` suffix until a free path
  is found.
- `copy_dir_all(src, dst)` — recursive directory copy.
- `refresh_panels(app)` — reloads both panels and restores valid selections.

## Error and Refresh Semantics

The operations return `io::Result<()>`, but they are not a transaction layer.
Copy and move collect per-item transfer failures into status and continue the
batch; their returned error is reserved for the final panel refresh. Delete
does the same for per-path removal failures. Mkdir and file creation can return
their direct filesystem errors. Popup handling and the main-view `report`
boundary translate operation errors into status instead of terminating the
event loop.

## Test Fault Injection

Fault injection is cfg-gated, in-memory state, not environment variables:

- `FORCE_RENAME_FAILURE` (`src/ops/file.rs`) forces the move fallback.
- `FORCE_COPY_DIR_ALL_STEP` (`src/utils.rs`) selects a recursive-copy failure
  point.
- `FORCE_REFRESH_ERROR` (`src/utils.rs`) forces `refresh_panels` to fail.

The controls are compiled under `cfg(any(test, tarpaulin))`; test-only setters
and mutexes serialize cases that mutate shared state.

## Related Code

- `src/ops/file.rs`
- `src/ops/mod.rs`
- `src/utils.rs`
- `src/popup.rs`
- `src/handlers/mod.rs` (compatibility facade only)
- `docs/specifications/command-popup-and-operations-modules.md`
