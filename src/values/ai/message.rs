//! Purpose: AI conversation turns and dual-panel workspace context snapshot.

/// Turn author in an AI conversation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
}

impl Role {
    #[must_use]
    pub const fn as_api_str(self) -> &'static str {
        match self {
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }

    #[must_use]
    pub const fn prefix(self) -> &'static str {
        match self {
            Self::User => "You",
            Self::Assistant => "AI",
        }
    }
}

/// One user or assistant message in the AI dialog transcript.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub role: Role,
    pub text: String,
}

/// Dual-panel working directory and selection context sent alongside each prompt.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WorkspaceContext {
    pub active_dir: String,
    pub opposite_dir: String,
    pub selected: Option<String>,
    pub marked: Vec<String>,
    pub os: &'static str,
}

impl WorkspaceContext {
    #[must_use]
    pub fn prompt_summary(&self) -> String {
        let mut parts = vec![
            format!("OS: {}", self.os),
            format!("Active directory: {}", self.active_dir),
            format!("Opposite directory: {}", self.opposite_dir),
        ];
        if let Some(selected) = &self.selected {
            parts.push(format!("Selected item: {selected}"));
        }
        if !self.marked.is_empty() {
            let preview: Vec<&str> = self.marked.iter().take(20).map(String::as_str).collect();
            parts.push(format!("Marked items: {}", preview.join(", ")));
        }
        parts.join("\n")
    }
}

#[cfg(test)]
mod tests;
