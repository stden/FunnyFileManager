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

Passwords are requested when connecting and kept only in memory, never in
profiles, logs or history. Imports discard saved passwords. The connection
dialog masks its password field.

## What this build does not do

- **SCP and FTPS.** Native connections currently support SFTP and FTP.
  Remote panels support copying, moving, deleting, editing and Linux permissions.
- **Import from Far's profile database, `pwd.json` or a KeePass vault.**
  Export the sessions from NetBox (*Export sessions*) and import the
  `.netbox` file.
- **Change drive's Del, Shift+Del, F3, F4, F9, Ctrl+R**, and quick search on
  the sessions list.

## The files

- The user store: `<config dir>/ffm/ffm.yml`, under `sessions`.
  On Windows this is `%APPDATA%/ffm/ffm.yml`; on Unix it is
  `$XDG_CONFIG_HOME/ffm/ffm.yml` or `~/.config/ffm/ffm.yml`.
  This is a private user file, never a project configuration. Git ignores
  `ffm.yml` and its lock file. Saving preserves other YAML sections, uses an
  exclusive lock and an atomic owner-only replacement, and refuses conflicting
  changes to server profiles. Comments are rewritten with the YAML document.
  If `ffm.yml` is absent, existing `sessions.json` profiles are read;
  the next profile save writes YAML and leaves the old JSON untouched.
  Invalid or duplicate profiles refuse saving until the file is repaired.
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
