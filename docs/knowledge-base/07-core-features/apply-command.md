<!-- Purpose: Documents Ctrl+G command substitution and its Far compatibility. -->

# Apply command (Ctrl+G)

The template runs once per marked entry, in panel order, or once for the cursor
entry when nothing is marked. Each expanded command and its output appear in
the Ctrl+O transcript. Processed entries are unmarked; Ctrl+M restores the
selection. Selection substitutions therefore contain the remaining marks at
each iteration, as in Far.

The syntax follows [Far's help](https://github.com/FarGroup/FarManager/blob/master/far/FarEng.hlf.m4)
and [metacharacter parser](https://github.com/FarGroup/FarManager/blob/master/far/fnparse.cpp).
Inserted text is never parsed for metacharacters a second time. `{}` is literal.

| Symbol | Substitution |
|--------|--------------|
| `!.!` | Filename with extension |
| `!` | Filename without its final extension |
| `` !` `` | Extension without the dot |
| `!-!`, `!~`, `` !`~ `` | Short filename, short stem, short extension |
| `!+!` | Short filename; restore the original long name if a program recreates the file under its short name |
| `!\`, `!/` | Current directory, long or short, with a trailing separator |
| `!=\`, `!=/` | Resolved directory (following links), long or short, with a trailing separator |
| `!:` | Drive or UNC share; filesystem root on Unix |
| `!?!` | File description |
| `!&`, `!&~` | Selected long or short names, all quoted |
| `!&q`, `!&~q` | Selected names with quotes only around whitespace; `Q` explicitly quotes every name |
| `!@...!`, `!$...!` | Temporary list of selected long or short names |
| `!#`, `!^` | Use the passive or active panel for subsequent substitutions |
| `![`, `!]` | Use the left or right panel for subsequent substitutions |
| `!!` | Literal exclamation mark |

Short names come from the Windows filesystem, not from the panel's synthetic
display aliases. If an alias is unavailable, the real name is used. On Unix,
short-name substitutions also use the real name. Far's last-dot rule applies
even to `.gitignore`: the stem is empty and the extension is `gitignore`.

## Lists and quoting

Lists require a closing `!`. For example, `!@!` is a list; `!@` is a bare stem
substitution followed by a literal `@`. Supported modifiers are:

| Modifier | Meaning |
|----------|---------|
| `Q` | Quote every entry |
| `F` | Include the full path |
| `S` | Change backslashes to forward slashes in the list |
| `A` | Native Windows ANSI encoding |
| `U` | UTF-8 |
| `W` | UTF-16LE |

Without an encoding modifier, Windows uses its OEM code page. Unix uses UTF-8
for the native encodings. The last encoding modifier wins. Lists have native
line endings and no BOM. Each list has a unique temporary filename and is
deleted after its command, including on failure.

Other substitutions do not add quotes, including the temporary list's path.
Quote paths in the template when they may contain spaces. Examples:

- `copy "!.!" "!.bak"` — backup the current file on Windows (`cp` on Unix).
- `echo "!\!.!"` — full path, replacing the former `{}` placeholder.
- `copy "!#!\!.!" "!^!.bak"` — copy the passive-panel cursor file to a backup
  named after the active entry.
- `tool --files "!@UFQ!"` — UTF-8 list of quoted full paths.

## Scope

This implements Far's file, path, selection and panel substitutions. Interactive
`!?title?initial!` parameter dialogs and their variables are not implemented.
Commands use FFM's bounded platform shell, with its existing timeout and output
capture, rather than Far-specific `far:` commands.

Implementation: `models/apply/` owns parsing and pure substitution;
`ops/apply/` captures filesystem data, writes lists and runs commands.
