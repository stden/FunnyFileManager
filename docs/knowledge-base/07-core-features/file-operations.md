<!-- Purpose: Documents copy, move, delete, mkdir, creation, and refresh behavior. -->

# 📋 File Operations

File-operation implementations are owned by `src/ops/file.rs`. Low-level path,
recursive-copy, and refresh helpers live in `src/utils.rs`;
`src/handlers.rs` only preserves compatibility imports.

## 📝 Source Selection

Copy, move, and delete act on all marked names when the active panel has marks.
Otherwise they act on the selected entry. With neither marks nor a selection,
they return without changing the filesystem.

## 🎯 Destination (F5/F6 dialog)

The dialog offers the other panel's folder; the field may be edited to any
path, relative to the active panel. What was typed means, following Far
(`ops/file/destination.rs`):

- an existing folder, a path ending in a separator (Far: "append backslash
  to create the destination folder"), or the destination of several marked
  items is a folder: each item goes inside it under its own name, and a
  missing folder is created on the way;
- anything else is the new name of the one item. F6 to `renamed.txt`
  renames the file, F5 to `run-copy.sh` makes a copy with that name, and a
  folder sent to a new name becomes that folder (its contents land directly
  in it). A name already taken goes through "Already existing files".

## 📋 Copy (F5)

- Copies from the active panel to the inactive panel directory.
- Calls `unique_destination` for every source, so an existing destination is
  not overwritten.
- Uses `copy_dir_all` for directories and `utils::copy_file` for files. Like
  `fs::copy` it gives the copy the source's permission bits (a script stays
  executable, a private file stays private), but it claims the destination
  with an exclusive create and removes a copy that failed half-way.
- Keeps the holes of a sparse file (one with fewer blocks allocated than its
  length, `cp --sparse=auto`'s test): it is copied in 64 KiB blocks and a
  block of zeros is skipped with a seek instead of written, so a 5 GiB disk
  image that allocates nothing still allocates nothing once copied. Other
  files go through `io::copy`, which lets the kernel copy (or share) the
  blocks. The copy itself still runs on the event-loop thread.
- Refuses to copy a directory into itself or one of its descendants, even
  when the nested destination does not exist yet.
- Refuses a FIFO, socket or device node with "not a regular file", whether it
  is the entry under the cursor or inside a copied folder. The type is checked
  before the source is opened and before the destination is created, because
  `open` on a FIFO waits for a writer and the copy runs on the event-loop
  thread (`utils::copy_file`).
- Refuses to copy or move a file or folder onto itself, whatever the
  "Already existing files" choice (Far's "copy/move onto itself" error): both
  panels start in the same directory, so this is the dialog's default.
  "Overwrite" would empty the source before reading it, and "Ask" would
  rename a moved file to `name_1`.
- The status line says "Copied" or "Moved", and adds "(existing files kept,
  new ones renamed)" only when an item was put beside an existing one under
  a new name.
- "Access rights" (`copy::AccessRights`): Default gives every copied file its
  source's permission bits and creates folders like any new folder there;
  Copy also restates the copied folder's own mode once its contents are in;
  Inherit leaves every file and folder the copy creates with the mode the
  kernel gave it at creation (the umask, or the folder's default ACL). No
  choice ever sets a bare 0666/0777 with `chmod`, which would skip the umask.
  A same-filesystem move is a rename and keeps the mode it had.
- "Preserve timestamps" applies to directories as well as files; the copied
  directory gets the source's modification time once its contents are in.
- Removes a partially written destination when an individual copy fails.
- Continues a marked-item batch after an individual failure, reports the
  successful and failed counts, clears marks, and then refreshes both panels.

## 📋 Move (F6)

- Chooses a non-colliding destination with `unique_destination`.
- Tries `fs::rename` first.
- On any rename error, falls back to copy followed by source deletion.
- Applies the same descendant guard, partial-copy cleanup, batch error
  collection, mark clearing, and final refresh as copy.

The fallback is not fully transactional. The source is removed only after its
copy succeeds, but a successful copy is not rolled back if source deletion
fails. Earlier successful items in a multi-item operation also remain changed.

## 📋 Delete (F8)

- `handle_delete_confirm` opens `Overlay::DeleteConfirm` with the selected or
  marked paths.
- Confirmation in `popup` calls `handle_delete`.
- Files use `fs::remove_file`; directories use `fs::remove_dir_all`.
- Per-path failures are collected into the resulting status instead of stopping
  the remaining deletions.
- Marks are cleared after the attempts, then both panels are refreshed. A
  refresh failure is propagated.

## 📁 Mkdir (F7)

- Creates `new_folder`, then `new_folder_N` up to the bounded search limit.
- Uses `fs::create_dir_all`.
- Refreshes both panels and propagates refresh errors.

## 📄 New File (Shift+F4)

- `Overlay::NewFileInput` collects the name; `popup` delegates creation to
  `ops::handle_create_file`.
- Empty or whitespace-only input is rejected through status.
- An existing path is reported and never overwritten.
- On successful creation, a panel refresh is attempted. The current
  implementation deliberately does not propagate that refresh error.

## 🧰 Utilities

- `unique_destination(base, filename)` — adds a `_N` suffix until a free path
  is found.
- `copy_dir_all(src, dst)` — recursive directory copy.
- `refresh_panels(app)` — reloads both panels and restores valid selections.
  A panel whose folder has been deleted moves to the nearest folder above it
  that still exists (`Panel::reload`), as Far's does; Enter on `..` and the
  drive menu walk up the same way. Any other reload failure keeps the last
  listing and is reported.

## 🧩 Error and Refresh Semantics

The operations return `io::Result<()>`, but they are not a transaction layer.
Copy and move collect per-item transfer failures into status and continue the
batch; their returned error is reserved for the final panel refresh. Delete
does the same for per-path removal failures. Mkdir and file creation can return
their direct filesystem errors. Popup handling and the main-view `report`
boundary translate operation errors into status instead of terminating the
event loop.

## 🧪 Test Fault Injection

Fault injection is cfg-gated, in-memory state, not environment variables:

- `FORCE_RENAME_FAILURE` (`src/ops/file.rs`) forces the move fallback.
- `FORCE_COPY_DIR_ALL_STEP` (`src/utils.rs`) selects a recursive-copy failure
  point.
- `FORCE_REFRESH_ERROR` (`src/utils.rs`) forces `refresh_panels` to fail.

The controls are compiled under `cfg(any(test, tarpaulin))`; test-only setters
and mutexes serialize cases that mutate shared state.

## 🔗 Related Code

- `src/ops/file.rs`
- `src/ops.rs`
- `src/utils.rs`
- `src/popup.rs`
- `src/handlers.rs` (compatibility facade only)
- `docs/specifications/command-popup-and-operations-modules.md`
