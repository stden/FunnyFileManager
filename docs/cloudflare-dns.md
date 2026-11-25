<!-- Purpose: Setup, use and limits of the Cloudflare DNS editor (F9 → Commands → Cloudflare DNS). -->
# Cloudflare DNS

The Cloudflare DNS editor lists the zones and DNS records of a Cloudflare
account, adds this machine as an `A` record, and edits the content, TTL and
proxy of existing records. It runs from **F9 → Commands → Cloudflare DNS**.

## Create a token

Create the token in the Cloudflare dashboard under **My Profile → API Tokens →
Create Token → Create Custom Token**:

1. Add the permission **Zone · DNS · Edit**.
2. Add **Zone · Zone · Read**, so the zone list can be read.
3. Under **Zone Resources**, choose **Include · Specific zone** and pick the
   zone to edit. Add one line per zone when several are needed.
4. Create the token and copy it once. Cloudflare does not show it again.

## Set the variable

The editor reads the token from the environment variable
`CLOUDFLARE_API_TOKEN`, sent as a bearer token. Set it in the terminal that
starts FunnyFileManager:

```sh
export CLOUDFLARE_API_TOKEN='paste-the-token-here'
```

```powershell
$env:CLOUDFLARE_API_TOKEN = 'paste-the-token-here'
```

Keep the token out of files in the project, and out of shared history. A
missing or blank variable is reported on the dialog's status line as
`set CLOUDFLARE_API_TOKEN`, with the variable name only; there is no prompt for
the token in this version.

## Using the dialog

| Key                   | Action                                                                     |
| --------------------- | -------------------------------------------------------------------------- |
| Up / Down             | Move in the focused list. A new zone loads its records.                    |
| Tab                   | Switch between the zone list and the record list.                          |
| Enter                 | On the zone list: switch to the records. On a record: open its edit form.  |
| `a`                   | Add this machine as an `A` record in the zone its hostname belongs to.     |
| `r`                   | Read the zones again, then the records of the selected zone.               |
| Tab, Up / Down (form) | Move between the fields of the form.                                       |
| Space (form)          | Switch the proxy, where the record type can be proxied.                    |
| Enter (form)          | Check the form and show the change for confirmation.                       |
| Enter (confirmation)  | Send the change. This is the only request that writes.                     |
| Esc                   | Step back: confirmation to form, form to the list, list closes the dialog. |

**Add this host.** The hostname comes from `/proc/sys/kernel/hostname` on
Linux, `COMPUTERNAME` on Windows, and `HOSTNAME` as a fallback. The zone is the
account zone with the longest name that ends the hostname, matched on a label
boundary: `example.com` holds `host.example.com`, but not `hostexample.com`. The
record name is the hostname relative to that zone, and the apex keeps the zone
name. The public IPv4 address comes from Cloudflare's trace page, which is read
without the token. A private, loopback or unspecified address is refused. When
no address is found, the form stays open with an empty content field and says
why.

**Validation.** Checks run before anything is sent:

- A record name is dot-separated labels of letters, digits and hyphens. A label
  is 1 to 63 characters and does not start or end with a hyphen; the whole name
  is at most 253 characters.
- The content of an `A` record is an IPv4 address; an `AAAA` record takes an
  IPv6 address; other types take some text.
- The TTL is `1` (automatic) or 60 to 86400 seconds.
- Only `A`, `AAAA` and `CNAME` records can be proxied.

## What it does

- Lists zones (50 a page) and the records of the selected zone (100 a page),
  reading every page the API reports.
- Creates an `A` record in a zone, and updates the content, TTL or proxy of an
  existing record. An update sends only the fields that changed.
- Shows each write as a confirmation: the zone, the record name, and each old
  and new value. Nothing is sent until Enter is pressed on it.
- Runs every request in the background, so the file panels keep responding.
  While a request is in flight the dialog shows a loading line and ignores the
  keys that would start another one.
- Reports Cloudflare's own error as its first message and code.

## What it never does

- It never deletes a record, and there is no delete request in the code.
- It never changes a record's type.
- It never writes without the confirmation step: the request is built only
  after Enter on the confirmation.
- It never retries after a rate limit. HTTP 429 is shown as a rate limit.
- It never follows a redirect, and never sends the token anywhere but the
  Cloudflare API host. The public address lookup sends no token.
- It never shows the token: not in the status line, the errors, the logs or
  the URLs.

## Limits

- Requests use HTTPS only, with a 15 second timeout, and a user agent of `ffm`.
- The zone and record lists are read in full, up to 100 pages; a longer list is
  reported as too large rather than shown in part.
- Only the content, TTL and proxy of a record are edited; names of existing
  records are fixed.
- Other record types can be listed, but their content is edited as plain text.

The code is in `src/cloudflare_dns` (client, codec, validation), state in
`src/models/cloudflare_dns`, the background job in
`src/application/cloudflare_jobs`, actions in `src/ops/cloudflare_dns`, keys in
`src/popup/cloudflare_dns` and drawing in `src/ui/cloudflare_dns`.
