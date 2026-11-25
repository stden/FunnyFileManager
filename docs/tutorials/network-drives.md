<!-- Purpose: Private YAML storage of discovered network drive mappings. -->
# Network drives in ffm.yml

On a live FFM startup a background discovery reads the local operating system's
network mount metadata and merges it into **network_drives** in the user's
`ffm.yml`. On Windows the file is `%APPDATA%/ffm/ffm.yml`.
On Linux it is `$XDG_CONFIG_HOME/ffm/ffm.yml`, or
`~/.config/ffm/ffm.yml` when XDG_CONFIG_HOME is unset.

Windows discovery reads current mapped network disks and persistent mappings
under the current user's registry profile, including remembered disconnected
drives. It uses PowerShell 7 (`pwsh.exe`) without contacting file servers.
Linux discovery reads SMB, NFS, SSHFS and WebDAV mounts from
`/proc/self/mounts`. Other platforms currently do not have automatic discovery.

For example, a private configuration can contain:

```yaml
network_drives:
  - path: 'Z:\'
    remote: '\\example.test\share'
    protocol: smb
  - path: /mnt/team
    remote: 'example.test:/export/team'
    protocol: nfs
```

Protocols are typed values: `smb`, `nfs`, `sshfs`, `web_dav`.
Absolute mount points and remote addresses are validated when the YAML is read.
Passwords and credentials are not collected. Startup discovery alone does not
automatically mount, reconnect or change permissions on any drive.

## One-press connect and editing

Use one of the following entry points to edit `network_drives:` or connect every
configured unmounted drive in a single action:

- **Connect all drives**: press `Alt+F1` / `Alt+F2` → `Connect network drives`,
  `F9 → Commands → Connect all network drives`, or run `:netdrives`
  (`:netdrives connect`). Mounting runs off the UI thread, skips drives that are
  already mounted, logs each drive (`SKIP`, `OK`, `FAIL`) to the `Ctrl+O`
  console, and refreshes local file panels when done.
- **Edit configuration**: `F9 → Commands → Edit network drives (ffm.yml)` or
  `:netdrives edit` opens `ffm.yml` in the built-in editor (creating a commented
  starter file if none exists yet).
- **Inspect configured drives**: `:netdrives list` opens a read-only viewer of
  the entries loaded from `ffm.yml`.

A refresh replaces the discovered mapping for the same local path and retains
other saved mappings, so offline entries are not erased. Remove an obsolete
entry from the private YAML if it should no longer be remembered.
Other sections, including server `sessions`, are preserved under the same
configuration lock and atomic save mechanism. Discovery failures are logged
without replacing the previous mappings.

The file and its lock are ignored by Git. Replayed sessions do not run discovery
or modify the host's mappings.
