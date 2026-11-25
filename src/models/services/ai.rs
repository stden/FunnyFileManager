//! Purpose: Scrollable AI chat dialog state supporting Anthropic, `OpenAI`, and Codex backends.

use std::sync::Arc;

use crate::lineedit::LineEditor;
use crate::values::ai::{
    AiError, AiProvider, AiRequest, AiResponse, AiSettings, ChatMessage, Role, WorkspaceContext,
};

/// Numbers AI requests for one dialog so replies from older asks are ignored.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AiRequestId(u64);

impl AiRequestId {
    fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }
}

/// Interactive AI assistant overlay state.
#[derive(Clone)]
pub struct AiDialog {
    pub(crate) generation: Arc<()>,
    pub(crate) latest: AiRequestId,
    pub(crate) provider: AiProvider,
    pub(crate) settings: AiSettings,
    pub(crate) messages: Vec<ChatMessage>,
    pub(crate) prompt: LineEditor,
    pub(crate) scroll: usize,
    pub(crate) last_command: Option<String>,
    pub(crate) busy: bool,
    pub(crate) queued: Option<(AiRequestId, AiRequest)>,
    pub(crate) status: String,
}

impl AiDialog {
    #[must_use]
    pub(crate) fn new(settings: AiSettings) -> Self {
        let provider = settings.initial_provider();
        let status = format!("{} ready — type a prompt and press Enter", provider.label());
        Self {
            generation: Arc::new(()),
            latest: AiRequestId::default(),
            provider,
            settings,
            messages: Vec::new(),
            prompt: LineEditor::new(""),
            scroll: 0,
            last_command: None,
            busy: false,
            queued: None,
            status,
        }
    }

    pub(crate) fn cycle_provider(&mut self) {
        self.set_provider(self.provider.next());
    }

    pub(crate) fn set_provider(&mut self, provider: AiProvider) {
        self.provider = provider;
        self.status = format!("Switched provider to {}", provider.label());
    }

    pub(crate) fn queue_prompt(&mut self, context: WorkspaceContext) -> bool {
        let text = self.prompt.text().trim().to_owned();
        if text.is_empty() {
            "Enter a prompt first".clone_into(&mut self.status);
            return false;
        }
        self.messages.push(ChatMessage {
            role: Role::User,
            text,
        });
        self.prompt.clear();
        self.scroll = 0;
        let id = self.latest.next();
        self.latest = id;
        self.busy = true;
        self.status = format!("Asking {}...", self.provider.label());
        self.queued = Some((
            id,
            AiRequest {
                provider: self.provider,
                messages: self.messages.clone(),
                context,
                settings: self.settings.clone(),
            },
        ));
        true
    }

    pub(crate) fn take_queued(&mut self) -> Option<(AiRequestId, AiRequest)> {
        self.queued.take()
    }

    #[must_use]
    pub(crate) const fn awaits(&self, id: AiRequestId) -> bool {
        self.latest.0 == id.0
    }

    #[must_use]
    pub(crate) const fn is_busy(&self) -> bool {
        self.busy || self.queued.is_some()
    }

    pub(crate) fn apply_reply(&mut self, reply: Result<AiResponse, AiError>) {
        self.busy = false;
        self.scroll = 0;
        match reply {
            Ok(resp) => {
                if let Some(cmd) = &resp.command {
                    self.last_command = Some(cmd.clone());
                }
                let usage = match (resp.input_tokens, resp.output_tokens) {
                    (Some(i), Some(o)) => format!(" ({i}in/{o}out)"),
                    _ => String::new(),
                };
                self.status = format!("{} [{}]{usage}", resp.provider.label(), resp.model);
                self.messages.push(ChatMessage {
                    role: Role::Assistant,
                    text: resp.text,
                });
            }
            Err(error) => {
                self.status = error.to_string();
            }
        }
    }

    pub(crate) fn clear(&mut self) {
        self.messages.clear();
        self.last_command = None;
        self.scroll = 0;
        self.status = format!("{} conversation cleared", self.provider.label());
    }
}

#[cfg(test)]
mod tests;
