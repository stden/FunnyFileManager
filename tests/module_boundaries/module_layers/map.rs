//! Purpose: The layers of the crate, lowest first, and the modules in each.
//!
//! The one place that says which layer a module is in. The rule that reads it
//! is `module_layers.rs`; `docs/knowledge-base/02-architecture/layers.md` says
//! what each layer is for.

/// The layers, lowest first. A module may use its own layer or one before it.
pub const LAYERS: [(&str, &[&str]); 7] = [
    (
        "foundation",
        &[
            "casefold",
            "console",
            "cycle",
            "encoding",
            "lineedit",
            "mask",
            "path_display",
            "scroll",
            "space_verdict",
        ],
    ),
    (
        "adapters",
        &[
            "associations",
            "clipboard",
            "clock",
            "config",
            "description",
            "diagnostics",
            "directory",
            "diskfree",
            "fileload",
            "file_write",
            "git",
            "highlight",
            "history",
            "i18n",
            "images",
            "keepass",
            "lsp",
            "mcp_clients",
            "packages",
            "panel_session",
            "performance",
            "platform",
            "process_table",
            "progress",
            "recording",
            "remote",
            "s3",
            "sessions",
            "stats",
            "terminal",
            "things",
            "token_usage",
            "usermenu",
            "utils",
            "windows",
            "wsl",
        ],
    ),
    (
        "state",
        &[
            "command",
            "complete",
            "copy",
            "kdbx",
            "models",
            "path_env",
            "quicksearch",
            "search",
            "status",
        ],
    ),
    ("presentation", &["image_render", "mouse", "prompt", "ui"]),
    ("operations", &["editor", "ops"]),
    ("interaction", &["action", "keymap", "menu", "popup"]),
    (
        "composition",
        &["application", "automation", "bootstrap", "handlers"],
    ),
];

/// Test-only helpers: not checked.
pub const IGNORED_MODULES: &[&str] = &["test_support"];

/// Upward uses that exist today, as (source, target). Each is tolerated until
/// it is removed; a listed use that is gone fails the check.
pub const UPWARD_DEBT: &[(&str, &str)] = &[
    ("config", "models"),
    ("diagnostics", "action"),
    ("diagnostics", "models"),
    ("diagnostics", "status"),
    ("directory", "models"),
    ("editor", "menu"),
    ("git", "models"),
    ("history", "command"),
    ("images", "models"),
    ("keepass", "kdbx"),
    ("lsp", "complete"),
    ("mcp_clients", "status"),
    ("models", "application"),
    ("models", "keymap"),
    ("models", "menu"),
    ("models", "mouse"),
    ("models", "prompt"),
    ("ops", "action"),
    ("ops", "menu"),
    ("packages", "prompt"),
    ("platform", "models"),
    ("process_table", "models"),
    ("progress", "copy"),
    ("recording", "models"),
    ("remote", "models"),
    ("s3", "models"),
    ("sessions", "models"),
    ("stats", "models"),
    ("terminal", "ui"),
    ("things", "models"),
    ("token_usage", "models"),
    ("ui", "action"),
    ("ui", "menu"),
    ("utils", "models"),
    ("windows", "models"),
    ("windows", "status"),
    ("wsl", "models"),
];

/// Modules on a loop with modules of their own layer. The check fails when the
/// real loops and this list differ, so the list shrinks as loops are broken and
/// grows only on purpose.
pub const CYCLIC_MODULES: &[&str] = &[
    "action",
    "application",
    "automation",
    "bootstrap",
    "command",
    "complete",
    "config",
    "editor",
    "fileload",
    "history",
    "menu",
    "models",
    "ops",
    "recording",
    "search",
    "status",
    "terminal",
    "utils",
];
