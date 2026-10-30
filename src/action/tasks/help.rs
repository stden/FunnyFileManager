//! Purpose: the rows of the task list's help page (`F1`).

/// The help page (`F1`): each row's keys, as Far writes them and separated by
/// `", "`, then what they do. Every key named here resolves to a command.
pub const HELP: &[(&str, &str)] = &[
    (
        "Up, Down, PgUp, PgDn",
        "Move the cursor; it stays on its process",
    ),
    ("Home, End", "First or last process"),
    ("Left, Right", "Scroll the Command column"),
    ("Ctrl+Left, Ctrl+Right", "Scroll the Command column by 20"),
    ("Space, Ins", "Mark the process and move down"),
    ("c", "Mark the process and its descendants"),
    ("U", "Unmark all"),
    ("*", "Invert the marks"),
    ("+, -", "Select or deselect by a name mask"),
    ("Enter, F3", "Process information"),
    ("F2", "Command line or name in the last column"),
    ("F4, \\", "Filter: fixed strings, | means or"),
    ("/, Alt+letter", "Search; F3 in the bar finds the next"),
    ("0-9", "Jump to a PID"),
    ("F5, t, Ctrl+T", "List or tree"),
    ("F6, Ctrl+F12, <, >", "Sort by menu"),
    ("P, M, T, N", "Sort by CPU%, memory, CPU time, PID"),
    ("Ctrl+F3, Ctrl+F6, Ctrl+F7", "Sort by name, memory, PID"),
    (
        "Ctrl+F8, Ctrl+F10, Ctrl+F11",
        "Sort by start, command, user",
    ),
    ("I", "Invert the order"),
    ("u", "Show the processes of one user"),
    ("K", "Show or hide kernel threads"),
    ("F7", "Set the nice value of the main thread"),
    ("]", "Nice -1, at once: runs sooner; needs root"),
    ("[", "Nice +1: runs later; asks unless root"),
    ("F8, Del", "Terminate or kill, after a confirmation"),
    ("F9, k", "Send a signal, after a confirmation"),
    ("F11, Z", "Hold or resume the refresh"),
    ("Ctrl+R, Ctrl+L", "Refresh now"),
    ("F1, h, ?", "This help"),
    ("Esc, F10, q, Ctrl+C, Ctrl+W", "Close the task list"),
];
