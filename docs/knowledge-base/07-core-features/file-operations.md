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
- "Already existing files: Ask" (the default) asks before anything is
  transferred (`ops/file/clashes.rs`, `Overlay::ExistingFile`): Far's
  "Destination already exists" warning shows the path and, for the new and
  the existing entry, the size and the modification time. Overwrite (`O`,
  the default button), Skip (`S`), Keep both (`K`: the new one lands beside
  under a `_N` name via `unique_destination`) or Cancel (`C`/Esc: nothing
  is copied). "Remember choice" (`E`) answers every clash still to come.
  Clashes are asked one at a time; the transfer runs once the last one is
  answered, with each source's answer as its own "Already existing files"
  choice. A copy onto itself is not asked about: it is refused as below.
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
  Unix hard-link aliases are refused too: a rename between them reports
  success without moving either name. "Ask" must not rename a moved file to
  `name_1` when the destination is the source itself, so it is never put
  to the "Destination already exists" question.
- A single-entry overwrite prepares the copy and its metadata in a temporary
  sibling before atomically replacing the destination (`ops/file/replace.rs`).
  A failed copy or metadata update leaves the existing entry intact. On Linux
  a file is staged with no name at all (`O_TMPFILE`, `utils/unnamed_file.rs`)
  and named `.ffm-copy-*` only once it is whole, an instant before the rename:
  a copy killed half-way leaves nothing, since the kernel frees an unnamed
  file with its last handle. Where that is not possible (Windows, other
  Unixes, NFS or FAT without `O_TMPFILE`, no `/proc`) and for a link, the
  staged entry is named from the start; FFM removes it after any failure it
  survives, but a kill can leave that hidden `.ffm-copy-*` file behind. When a
  rename would change what the destination *is* — it has other hard links,
  another user owns it, or its folder takes no new entry — a copy of a file
  over a file is written through the file instead (`utils::rewrite_file`,
  the rule `file_write::replacing_changes_identity` the editor follows too),
  keeping its inode, owner, mode and other names, as `cp` does. Everything
  that can be refused is met before the old contents go: the source must
  open as a regular file, the destination for writing, a kept time or mode
  must be one the file would take, and the new contents must fit in the free
  space. Only an I/O failure or a kill during the write itself can leave that
  file cut short — the price of keeping its identity. An overwrite keeps the
  replaced file's mode unless "Access rights: Copy" asks for the source's; a
  move replaces the name, as `mv` does. A failure that belongs to the
  destination names the destination (`ops/file/failed_at.rs`). Moves
  try an atomic rename first; only across filesystems do they copy, and that
  fallback removes the source only after the replacement succeeds.
- "Overwrite", "Skip" and "Only newer" never remove what the transfer does
  not bring (`ops/file/merge.rs`). A folder onto a folder of the same name is
  merged into it, as Far, mc and Explorer do: each entry inside follows the
  same choice, "Skip" and "Only newer" are decided file by file (Skip leaves
  the files already there, not the whole folder), and what only the
  destination holds stays; the existing folder keeps its own times and
  rights. A moved folder is removed once empty, and kept when something in
  it was skipped. A file or link and a folder never replace one another
  ("a file or link cannot overwrite a folder", "a folder cannot overwrite a
  file or link"), and a
  folder is never sent onto a folder that holds it (`pkg/pkg` onto `pkg`),
  which would take the source with it.
- A name that is not UTF-8 is shown lossily (`caf\xE9.txt` as `caf�.txt`),
  but every operation reaches the file by its name on disk
  (`DirEntry::disk_name`). F8 on such a row never touches another file shown
  the same way; a mark shows on, and reaches, every row with its name
  (`Panel::action_paths`).
- A name is taken when anything is there, a link that points nowhere
  included (`utils::name_is_taken`, `symlink_metadata`): "Skip" leaves such a
  link alone, "Ask" asks about it (Keep both puts the entry beside it), "Overwrite" replaces the link
  itself, and Shift+F6 refuses to rename onto it. `Path::exists` follows the
  link and used to call the name free, so a move renamed over it.
- The status line says "Copied" or "Moved", adds ", N skipped" for every
  entry the choice left alone — inside a merged folder too, at any depth, so
  a move that left files behind says so — and adds "(existing files kept,
  new ones renamed)" only when an item was put beside an existing one under
  a new name.
- "Access rights" (`copy::AccessRights`): Default gives every copied file its
  source's permission bits and creates folders like any new folder there;
  Copy also restates the mode of every folder the copy makes, each once its
  contents are in (`CopyRules::folder_modes`), so a private 0700 subfolder
  stays private;
  Inherit leaves every file and folder the copy creates with the mode the
  kernel gave it at creation (the umask, or the folder's default ACL). No
  choice ever sets a bare 0666/0777 with `chmod`, which would skip the umask.
  A same-filesystem move is a rename and keeps the mode it had; a move
  across filesystems, which has to copy, keeps what that rename would have
  kept — every file's and folder's mode and access and modification times,
  as `mv` does — unless Inherit asks for the destination's rights
  (`CopyRules::MOVE`, `CopyPrompt::rules`).
- "Preserve all timestamps" (`CopyRules::times`) gives every file, folder
  and link the copy makes its source's access and modification times, at
  any depth. A file gets them through its handle before its mode is set; a
  folder once its contents are in; a link its own times (`AT_SYMLINK_NOFOLLOW`),
  never its target's (`utils/keep_times.rs`).
- Removes a partially written destination when an individual copy fails.
- Stops a marked-item batch at the first entry it cannot copy, as Far does,
  and shows why (see "Stopped by an error" below); the status line counts
  what was copied. Marks are cleared and both panels refreshed either way.

## 📋 Move (F6)

- Chooses a non-colliding destination with `unique_destination`.
- Tries `fs::rename` first.
- Only a rename across filesystems (`EXDEV`, `io::ErrorKind::CrossesDevices`)
  falls back to copy followed by source deletion, as `mv` does. Any other
  rename failure (no write permission on the source's folder, a busy mount
  point) is reported and leaves both sides untouched (`ops::file::rename_within`).
- Applies the same descendant guard, partial-copy cleanup, batch error
  collection, mark clearing, and final refresh as copy.

The fallback is not fully transactional. The source is removed only after its
copy succeeds, and then only what the copy holds (`ops/file/remove_moved.rs`):
an entry that appeared in the source meanwhile stays, and so does anything
that will not go (a read-only or foreign subfolder). The removal carries on
past both and the item is reported as moved *and* as an error naming what
stayed: "Moved 1 item(s), 1 error(s): …/rootsub/r.txt: the copy is
complete, but this stayed in the source: Permission denied". A successful
copy is never rolled back. Earlier successful items in a multi-item
operation also remain changed.

## 📋 Delete (F8)

- `handle_delete_confirm` opens `Overlay::DeleteConfirm` with the selected or
  marked paths. For one entry the question names its kind as the panel
  listed it, as Far does — "Do you wish to delete the folder?", "…the link?",
  "…the file?" — and so does Alt+Del's wipe question; several are counted.
- Confirmation in `popup` calls `handle_delete`.
- Files use `fs::remove_file`; directories use `fs::remove_dir_all`.
- Stops at the first entry that will not go and shows why (see "Stopped by
  an error" below); nothing after it is touched. A read-only file is
  removed, as `remove_dir_all` removes one inside a folder.
- Marks are cleared after the attempts, then both panels are refreshed. A
  refresh failure is propagated.

## 📁 Mkdir (F7)

- The Make folder dialog takes a name or a path, as Far's does: `p1/p2/p3`
  makes every folder on the way (`popup/name_prompt.rs`), the last one
  claimed exclusively; a name that is taken, a dangling link included, is
  refused with "already exists". The cursor lands on the first folder of the
  path. Rename (Shift+F6) and Link names still refuse a separator.
- `handle_mkdir` (the facade's F7 without a dialog) creates `new_folder`, then
  `new_folder_N` up to the bounded search limit.
- Refreshes both panels and propagates refresh errors.

## 📄 New File (Shift+F4)

- `Overlay::NewFileInput` collects the name; `popup` delegates creation to
  `ops::handle_create_file`.
- Empty or whitespace-only input is rejected through status.
- An existing path is reported and never overwritten.
- On successful creation, a panel refresh is attempted. The current
  implementation deliberately does not propagate that refresh error.

## 📊 Progress (F5, F6, F8)

A copy, move or delete that runs longer than 100 ms shows Far's progress
dialog: the operation, the file or folder it is at, a bar with its
percentage, `Files: done of total` (and the sizes for a copy or move) and the
elapsed time. It is redrawn at most once per 100 ms (`progress::SHOWN_EVERY`),
so the file name changes at most ten times a second.

- A copy and a delete first measure the selection without following links
  ("Scanning"); the bar counts bytes for a copy and files for a delete.
- A move measures nothing up front, since a rename takes no time: its bar
  counts the selected entries, and a move across filesystems adds the bytes
  of what it has to copy when it comes to it.
- A file is copied in 4 MiB `io::copy` chunks (still `copy_file_range`
  underneath), so the bar moves inside a big file too.
- With the dialog shown, a folder is deleted entry by entry
  (`ops/file/entry.rs::remove_tree_reporting`) instead of with one
  `remove_dir_all`, never following a link; whatever that walk cannot remove
  gets a `remove_dir_all`, whose error is the one reported. Like the move
  fallback's removal, the walk goes by path.
- The operation still runs on the UI thread and cannot yet be cancelled
  with Esc.
- The reports are thread-local and cost nothing without a painter, which only
  the production session installs: tests install a recording painter
  (`ops/file/tests_progress.rs`).

## ⛔ Stopped by an error (F5, F6, F8)

Copy, move and delete stop at the first entry they cannot handle and open
`Overlay::OperationFailed` (`models/overlay/operation_failed.rs`), Far's red
"Cannot copy / move / delete" box: the path (named by where inside a folder
it failed, `utils::FailedAt`), the system's error, why in plain words when
it is a refusal, and "Stopped: N done, M not started". The status line
still counts what was done.

Why (`ops/file/denied.rs`, `Denial`):

- in use by another program (Windows' sharing or lock violation);
- read-only (Windows attribute);
- the account's rights: "an administrator may have them" when FFM is not
  elevated, "denies even an administrator" when it is;
- on Unix: the folder holding it takes no change from this user, another
  user owns it, or its mode does not allow it.

**Retry as administrator** (Windows, FFM not elevated, a refusal an
administrator may get past; `ops/file/elevated/`). The button — the default,
also `A` or a click — asks Windows for the rights (UAC) and starts a helper:
this same program with `--elevated-helper <pipe> <ffm pid>`, hidden, running
as an administrator. It redoes the failed entry and those not started, with
the F5/F6 dialog's settings, and stops where FFM would. The helper stays for
the rest of the session: every later refusal is handed to it at once, with
no dialog and no second prompt. It ends when FFM exits and the pipe closes.

- FFM creates the pipe (`platform/windows/pipe.rs`, first and only
  instance, local clients only, a random name) before Windows starts the
  helper, and lets it in only if the process connected is the one Windows
  started; the helper serves only if the process at the other end is the
  FFM whose id it was given. A malformed request is refused, never read as
  a delete.
- One JSON document a line (`elevated/protocol.rs`): FFM sends an
  `ElevatedRetry`, the helper answers with progress snapshots, which FFM
  draws in its own progress dialog, then an `Outcome`.
- Saying no to the prompt leaves "Administrator rights were not granted" in
  the status line. What even the administrator could not do is shown in the
  same box, with nothing more to offer.
- The quick F5/F6 (no dialog) offers no retry: it has no settings to send.

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
Copy, move and delete stop at the first item that fails, report it in the
status line and in `Overlay::OperationFailed`, and return `Ok`; their returned
error is reserved for the final panel refresh. Mkdir and file creation can return
their direct filesystem errors. Popup handling and the main-view `report`
boundary translate operation errors into status instead of terminating the
event loop.

An error names the path it happened at wherever the failing step knows it
(`utils::FailedAt`, carried inside the `io::Error`): F3 on a file that cannot
be read, Enter or `cd` on a folder that cannot be, and a copy's destination
that refuses the new file or folder. `Status::of_error` turns such an error
into `Status::ErrorAt` ("Error: …/secret.txt: Permission denied") and a
copy's batch reports that path instead of the source's; any other error
stays `Status::Error`.

## 🧪 Test Fault Injection

Fault injection is cfg-gated, in-memory state, not environment variables:

- `FORCE_RENAME_FAILURE` (`src/ops/file.rs`) makes every rename answer as one
  across filesystems (`CrossesDevices`), forcing the move fallback.
- `FORCE_COPY_DIR_ALL_STEP` (`src/utils.rs`) selects a recursive-copy failure
  point.
- `FORCE_REFRESH_ERROR` (`src/utils.rs`) forces `refresh_panels` to fail.
- Thread-local, for the steps root never sees fail: `FORCE_RENAME_DENIAL`
  (a rename refused like `EACCES`), `FORCE_REMOVAL_REFUSAL` (one entry of a
  moved source that will not go, `ops/file/remove_moved.rs`) and
  `FORCE_TIMES_FAILURE` (a copy's times that cannot be kept,
  `utils/keep_times.rs`).

The controls are compiled under `cfg(any(test, tarpaulin))`; test-only setters
and mutexes serialize cases that mutate shared state.

## 🔗 Related Code

- `src/ops/file.rs`
- `src/ops.rs`
- `src/utils.rs`
- `src/popup.rs`
- `src/handlers.rs` (compatibility facade only)
- `docs/specifications/command-popup-and-operations-modules.md`
