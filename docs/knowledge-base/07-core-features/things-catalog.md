<!-- Purpose: The Things catalog dialog: its configuration, what it reads, and how machines become sessions. -->

# Things catalog

F9 → Commands → `Things catalog` (no hotkey) lists the machines of the Things asset catalog and imports them as SFTP sessions in the stored-sessions list, so a whole fleet can be added at once. The sessions are the same ones the [NetBox import](netbox-sessions.md) makes.

## Configuration

Both values are read from the environment when the dialog opens. Neither is written to `ffm.yml`, the console, the status line or any log.

| Variable | Value |
|----------|-------|
| `THINGS_URL` | `https://things.example.com`. It must start with `https://` and carry no user name, password, query (`?`) or fragment (`#`). Trailing slashes are trimmed. |
| `THINGS_API_TOKEN` | `<token>`, a bearer token sent as `Authorization: Bearer <token>`. |

If either variable is missing or invalid, the dialog names the variable and makes no request.

## Reading the catalog

- One request, `GET {THINGS_URL}/api/v1/machines`, is made each time the dialog opens, on a worker thread so the screen never waits for it. It is HTTPS only, does not follow redirects, times out after 15 seconds and refuses a body over 8 MiB.
- The body may be a JSON array, an object `{"success", "data", "total_count", …}`, or an object `{"machines": […]}`. Any other shape is an unexpected response. Unknown fields are ignored, and a machine missing an optional field still lists.
- When `total_count` is larger than the number of machines returned, the dialog title and the status line say `showing N of M`. Nothing is cut off without saying so.
- Failures show a generic message that names what to check (`THINGS_URL`, the token, the network). The token never appears in a message.

## The dialog

Machines are listed by name, each with its host (the hostname, or the IP address when the hostname is blank), its OS, its environment and its state.

| Key | Action |
|-----|--------|
| Letters and digits | Add to the filter. The filter ignores case and matches the name, hostname, IP address, environment, location and tags. |
| Backspace | Remove the last filter character. |
| Up, Down, PgUp, PgDn, Home, End | Move the cursor. |
| Space | Mark or unmark the row under the cursor. Space is never typed into the filter. |
| Enter | Import the marked machines or, when none are marked, every machine the filter shows. |
| Esc or F10 | Close the dialog. |

Every key is taken by the dialog, so nothing typed in it reaches the panels behind.

## Import

Each chosen machine becomes an SFTP session on port 22 with password login and no start directory:

- The session is named after the machine, and its host is the hostname, or the IP address when the hostname is blank.
- The password is asked for when connecting. It is never stored.
- The session has no user name. Set one in the session dialog (F4) before connecting.
- The session records the Things machine id as its origin, so a later import refreshes it.
- A session made by hand with the same name is kept and reported as skipped.
- A machine whose name or host is not valid is skipped, and its reason goes to the console.

The status line reports `Things: N imported, M skipped`.

## Limits

- Only machines are read. Other Things entities are not browsed.
- The catalog is read again each time the dialog opens; there is no cache.
- Reads happen only in a live session. A test or a replayed session never reaches the network.
