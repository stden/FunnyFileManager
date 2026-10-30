<!-- Purpose: Documents NetBox-style stored sessions: the panel, the dialog, the .netbox import, and what this build does not do yet. -->

# 🌐 NetBox Stored Sessions

Far Manager's NetBox plugin keeps SFTP, SCP and FTP connections as *stored
sessions*. FFM keeps the same list, lets you manage it from a panel, and
imports NetBox's own export. The design and its later steps are in
`docs/superpowers/specs/2026-09-22-change-drive-and-netbox-design.md`.

## What works

- **Open the list**: Alt+F1 / Alt+F2 → `NetBox`, F11 → *NetBox: stored
  sessions*, or `:sessions`. The panel is titled `NetBox: Stored sessions [/]`;
  its rows are `..` then the sessions by name, with `user@host` beside a
  session whose name is something else. The info row under them counts the
  sessions; while the panel is active it is the status line, as a file
  panel's is, until the cursor moves on — so a refusal, a mistake in the
  dialog or a sessions file that did not load is said there.
- **Keys on the list**: Up/Down/Home/End/PgUp/PgDn move; Enter on `..` returns
  to the files; F3 shows a session's settings; F4 edits one, Shift+F4 makes a
  new one; F5 copies one under a new name; F8 deletes one after asking. Keys
  that would act on the file list hidden beneath (F7, marking, Alt+Del,
  Ctrl+A …) are refused with a status line; Tab, Alt+F1/F2, F9–F12 and the
  command line keep working. The mouse selects a row; a double click is Enter.
- **The session dialog**: name (blank = NetBox's `user@host[:port]`),
  protocol (SFTP, SCP, FTP, FTPS), host, port (blank = the protocol's; it
  follows the protocol while it is still the old default), user, login
  (password, key file, SSH agent), key file, start directory. Every field is
  checked when you press Enter, and the dialog stays open on a mistake.
- **Import**: F11 → *NetBox: import from .netbox file…* asks for the file,
  offering the `.netbox` file under a file panel's cursor;
  `:sessions import netbox [path]` does the same from the command line. The
  status bar says `N imported, M skipped`; the console transcript (Ctrl+O)
  lists every skipped entry with its reason.

## Passwords

No password is ever stored, logged, shown or written to the history. The
dialog has no password field, `sessions.json` has none, and an import counts
the saved passwords it found and leaves them out — the transcript says how
many. A password is to be asked for when connecting.

## What this build does not do

- **Connect.** There is no SFTP/SCP/FTP transport yet: Enter on a session
  says so, and the session stays stored. The remote panel it will open — its
  listing, Enter into a directory, `..` out of the host — is built and tested
  against the in-memory host in `src/remote/fake.rs`.
- **Import from Far's profile database, `pwd.json` or a KeePass vault.**
  Export the sessions from NetBox (*Export sessions*) and import the
  `.netbox` file.
- **Change drive's Del, Shift+Del, F3, F4, F9, Ctrl+R**, and quick search on
  the sessions list.

## The files

- The store: `<config dir>/ffm/sessions.json` — `$XDG_CONFIG_HOME` (else
  `~/.config`) on Unix, `%APPDATA%` on Windows — written atomically, mode
  `0600`, at most 1 MiB, with the guards of `settings.yaml`: a symbolic link
  (a dotfiles repository) is written through and stays a link, and a link to
  nothing, a file with other hard links, another user's file or a read-only
  one is refused (`cannot write the sessions file: sessions.json belongs to
  another user; FFM will not replace it`). The histories, the bookmarks and
  the panel session are written the same way. A file that exists but cannot
  be read, parsed or checked (a hand edit gone wrong, two sessions with one
  name, another user's 0600 file) is left exactly as it is: the list starts
  empty, the status line says why (`sessions.json did not load (Permission
  denied …), so it is not overwritten; fix or move it first`), and every
  change is refused until the file is fixed or moved. A save or import whose
  serialized JSON would exceed 1 MiB is refused before writing, preserving
  both the previous file and the session's committed entries.
- The import: NetBox's XML, `<NetBox><Sessions><Session name="…">` with
  `HostName`, `PortNumber`, `UserName`, `FSProtocol` (a number or a name),
  `Ftps`, `RemoteDirectory`, `PublicKeyFile` and `Password`, every value
  URL-encoded. The file is treated as untrusted: at most 1 MiB, no DTD (so no
  entity is expanded and nothing outside the file is read), elements nested
  at most 32 deep (checked before the XML is parsed, since the parser
  recurses once a level), at most 100 000 XML nodes and 1 000 sessions;
  every value is decoded and checked into the
  session's own types, and an entry with any bad value — a host with a space,
  port 0, a relative directory, a setting given twice — is skipped with the
  reason. A session the user made or changed keeps its name against an
  import; one an earlier import wrote is refreshed.

## Example

```xml
<NetBox version="2.1">
  <Sessions>
    <Session name="demo@example.test">
      <HostName>example.test</HostName>
      <PortNumber>2222</PortNumber>
      <UserName>demo</UserName>
      <FSProtocol>SFTP%20(SCP)</FSProtocol>
      <RemoteDirectory>%2Fvar%2Fwww</RemoteDirectory>
    </Session>
  </Sessions>
</NetBox>
```

`:sessions import netbox demo.netbox` stores `demo@example.test` (SFTP, port
2222, start directory `/var/www`).

## Where the code is

`src/models/session.rs` (the session and its checked values),
`src/sessions.rs` (the store), `src/sessions/import/netbox.rs` (the importer),
`src/remote.rs` (the `RemoteFs` seam), `src/ops/sessions.rs`,
`src/ops/remote.rs`, `src/popup/sessions.rs`, `src/popup/remote.rs`,
`src/ui/sessions.rs`, `src/ui/remote.rs`. The module map is in
`docs/knowledge-base/02-architecture/module-structure.md`.
