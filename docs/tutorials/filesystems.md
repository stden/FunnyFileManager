<!-- Purpose: Explain how panels read local disks, listings, and file copies. -->

# Local filesystems

A panel is one directory on a disk the operating system already mounted. FFM
does not invent a virtual filesystem for local files. `directory::read` asks
the OS for that directory, and the panel draws the snapshot it got back.

## Drives

`Alt+F1` opens the drive list for the left panel. `Alt+F2` opens it for the
right panel. The rows are built once, when the dialog opens.

| Host | What the list contains |
|---|---|
| Linux and macOS | One local row, `/` |
| Windows | One row for each drive letter returned by `GetLogicalDrives`, such as `C:\` |

Enter, or the row's own key, opens that drive. If either panel is already
somewhere on it, the chosen panel opens in that directory rather than at the
drive root. The active panel is checked first. A drive neither panel is on
opens at its root. An unreadable path does not stick: the panel stays where
it was, and the status line reports the failure.

## What one listing contains

The read opens the directory first. If that open fails, the previous listing
stays. Entries whose metadata cannot be read are skipped. Everything else
becomes one row:

| Fact | Where it comes from |
|---|---|
| Name | The directory entry |
| Kind | Directory, file, symlink to a directory, symlink to a file, or a broken symlink |
| Size | `len` of that entry. A symlink reports the link, not the target |
| Modified, created, accessed | The same metadata. Created and accessed are omitted when the OS has no answer |
| Link count | Unix `st_nlink`. Other hosts report 1 |
| Owner | Resolved while the directory is read, then stored on the row |
| Allocated | Unix `st_blocks` times 512. Elsewhere, the file length |
| Description | A line from the first existing file among `Descript.ion` and `Files.bbs` |
| Free space | Captured once for the directory, not per row |

`..` is added by FFM. It is not a second question to the OS about an entry
named dot-dot. Its time comes from the parent directory when that directory
can be stated.

The Descriptions and Long descriptions column layouts read `descript.ion`,
unless F9 → Options → File descriptions names another file. That setting does
not change the `Descript.ion` / `Files.bbs` search used for the row field
above. Quick view of a folder, when InfoPanel settings is `on`, reads the
semicolon-separated names from Folder description files. That setting's
default is `README.md;README.txt`.

A symlink keeps its own timestamps. Its kind follows the target: a link to a
directory is `SymlinkToDirectory`, a link to anything else that can be stated
is `SymlinkToFile`, and a target that cannot be stated is `BrokenSymlink`.
The executable mark follows the target on Unix (any of the `0111` mode bits
on a file) and, on Windows, the name's extension: `exe`, `com`, `bat`, `cmd`,
`ps1`, or `msi`.

## Hidden names

Two different facts get the same treatment when the panel is told to hide them:

- a name that starts with `.`
- a name recorded as hidden while the directory was read

On Windows that record is the hidden file attribute. On Unix the read does
not mark names hidden by itself, so a leading dot is the whole signal.

F9 → Options → Panel settings is `on` or `off`. `on` shows those names. `off`
removes them from the listing. The default value is `on`. Directories stay
visible through a filename mask. The mask applies to files. `Ctrl+I` is still
a placeholder dialog; the working mask is the one set from the F9 row, stored
on the panel, and reapplied the next time the directory is read.

## Copy, move, and names

Copy and move land in the other panel's directory. They do not replace a
file that is already there. `report.txt` becomes `report_1.txt`, then
`report_2.txt`, and so on, up to 9999 attempts. The check uses
`symlink_metadata`, so a name occupied by a symlink counts as taken.

A move tries an atomic rename. Across devices that fails, and the move
continues as copy plus delete. Marks are cleared when the panel actually
changes directory. A refresh keeps marks whose names are still in the new
listing.

## What this is not

The panel does not watch the directory. `Ctrl+R` reads it again. It does not
follow a directory symlink into a synthetic tree of its own: Enter on a
directory, including a symlink to one, navigates to that path and reads it.
S3 buckets are not directories in this listing. That split is the subject of
[S3](s3.md).
