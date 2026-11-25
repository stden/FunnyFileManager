//! Purpose: Extract a runnable shell command from fenced code blocks in AI responses.

const SHELL_TAGS: &[&str] = &[
    "bash",
    "sh",
    "zsh",
    "shell",
    "pwsh",
    "powershell",
    "cmd",
    "bat",
];

/// Return the first non-comment command line from a fenced shell block in `markdown`.
#[must_use]
pub fn first_shell_command(markdown: &str) -> Option<String> {
    let mut in_block = false;
    let mut is_shell = false;
    for raw_line in markdown.lines() {
        let trimmed = raw_line.trim();
        if let Some(tag) = trimmed.strip_prefix("```") {
            if in_block {
                in_block = false;
                is_shell = false;
            } else {
                in_block = true;
                let lang = tag.trim().to_ascii_lowercase();
                is_shell = SHELL_TAGS.contains(&lang.as_str());
            }
            continue;
        }
        if in_block && is_shell {
            if let Some(cmd) = normalize_command_line(trimmed) {
                return Some(cmd);
            }
        }
    }
    None
}

fn normalize_command_line(line: &str) -> Option<String> {
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let stripped = line
        .strip_prefix("$ ")
        .or_else(|| line.strip_prefix("> "))
        .unwrap_or(line)
        .trim();
    (!stripped.is_empty()).then(|| stripped.to_owned())
}
