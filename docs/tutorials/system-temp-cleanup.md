<!-- Purpose: Explains the scope, preview, confirmation and limits of temporary-file cleanup. -->
# Clean temporary files

Open **F9 → Commands → Clean temporary files**. FFM scans the operating
system's temporary directory in a background worker and shows a preview.
The dialog names the temporary folder, the number of eligible files, their
total size and the number of entries skipped during scanning.

Only regular temporary files with a modification time older than seven days
are eligible. Recent files, symbolic links and directories are preserved.
The scan can include files in ordinary subdirectories of the temporary folder;
it does not remove those subdirectories or the temporary root.

The command adds no targets outside this temporary root. Application caches,
browser caches, package caches, Cargo output and downloads elsewhere are not
scanned. It does not invoke an operating-system disk-cleanup utility.

On Windows, live cleanup supports the current user's standard
`%LOCALAPPDATA%\Temp` under their home folder, and only when it matches the
operating system's temporary directory. Shared, system-wide and custom
temporary roots outside that scope are refused.

On Unix, other users' files and subdirectories are skipped. Traversal stays
on the temporary root's filesystem and skips mounts on other devices.

## Preview and confirmation

The preview selects **Cancel** by default. Press **Esc** to leave the dialog,
or **Enter** to accept that default without deleting anything.

To clean the previewed files:

1. Select **Clean up** with Tab or the arrow keys and press Enter.
2. Review the separate confirmation, which selects **No** by default.
3. Press **Y** or explicitly select **Yes** and press Enter.

Enter on the default **No** returns to the preview without deletion.
After confirmation, deletion runs in the background. The result shows the
logical size and number of files removed, and skipped or failed removals.

The size is the sum of file lengths, not a measurement of reclaimed disk space.
Disk recovery can differ for sparse files, hard links, compressed files and
files still held open by another process. The reported logical size must not
be read as a guarantee that the same amount of storage became available.

Deleting these temporary files is permanent. The feature has no restore
operation. Closing a deletion progress dialog does not undo files already
removed.

## Files that changed or cannot be removed

Before deletion FFM checks each previewed file again, including its identity,
size and modification time. A replaced, modified or no-longer-eligible file is
skipped. The temporary root and parent directories are also rechecked.

Files that cannot be opened or locked are skipped or reported as failures.
The lock is advisory on Unix and is not a complete test for whether another
program has a file open. Keep files needed by a running program out of cleanup;
a file's age alone does not prove that no process still uses it.

A refused scan does not trigger a wider cleanup. Unsafe or unavailable
temporary-directory sources and scan limits produce an error in the dialog.
Replayed sessions do not obtain the real system temporary-directory capability.

## Tests

BDD scenarios use a temporary fixture directory with an old file, a fresh file,
an empty subdirectory and separate old cache/download fixtures. They drive the
actual F9 menu, confirmation keys, background worker and rendered result.
They verify cancellation, the default No, explicitly confirmed deletion,
preservation of files outside the root and retention after a previewed file
changes. They never clean the developer's real system temporary directory.
