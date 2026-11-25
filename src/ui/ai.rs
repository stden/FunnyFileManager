//! Purpose: Render the Warp-like AI assistant chat overlay (Anthropic, `OpenAI`, Codex).

#[cfg(test)]
mod tests;

use ratatui::Frame;

use super::dialog::{self, Ground, Row};
use super::overlays::clip_cells as clip;
use crate::models::services::ai::AiDialog;
use crate::mouse::DialogMouse;
use crate::values::ai::AiProvider;

pub(super) fn render(f: &mut Frame<'_>, tracker: &mut DialogMouse, dialog: &AiDialog) {
    let max_w = usize::from(f.area().width.saturating_sub(12)).clamp(36, 96);
    let max_h = usize::from(f.area().height.saturating_sub(15)).clamp(4, 14);
    let (providers, transcript) = (
        provider_bar(dialog.provider),
        transcript_window(dialog, max_w, max_h),
    );
    let cmd_hint = dialog
        .last_command
        .as_ref()
        .map(|cmd| clip(&format!("Cmd (F5/Ctrl+X): {cmd}"), max_w));
    let (status, keys) = footer_lines(dialog, max_w);
    let mut rows = vec![Row::Plain(&providers), Row::Divider];
    rows.extend(transcript.iter().map(|line| Row::Plain(line)));
    if let Some(hint) = &cmd_hint {
        rows.extend([Row::Divider, Row::Plain(hint)]);
    }
    rows.extend([
        Row::Divider,
        Row::Label("Prompt (Enter send, Esc close):"),
        Row::line(&dialog.prompt, true, false),
        Row::Divider,
        Row::Plain(&status),
        Row::Plain(&keys),
    ]);
    if let Some(pos) =
        dialog::render_tracked(f, tracker, "AI Assistant", Ground::Prompt, &rows, &[])
    {
        f.set_cursor_position(pos);
    }
}

fn footer_lines(dialog: &AiDialog, max_w: usize) -> (String, String) {
    (
        clip(&dialog.status, max_w),
        clip(
            "Tab/Ctrl+P Provider  Alt+1/2/3  F5 Command  F2 Save  F8 Clear  Up/Dn Scroll",
            max_w,
        ),
    )
}

fn provider_bar(active: AiProvider) -> String {
    let items: Vec<String> = [AiProvider::Anthropic, AiProvider::OpenAi, AiProvider::Codex]
        .iter()
        .map(|&provider| {
            if provider == active {
                format!("[{}]", provider.label())
            } else {
                format!(" {} ", provider.label())
            }
        })
        .collect();
    format!("Providers: {}", items.join(" | "))
}

fn transcript_window(dialog: &AiDialog, max_w: usize, max_h: usize) -> Vec<String> {
    if dialog.messages.is_empty() {
        return vec![clip(
            "Ask about files, commands, or errors — current panels are included automatically.",
            max_w,
        )];
    }
    let mut wrapped = Vec::new();
    for msg in &dialog.messages {
        wrapped.push(clip(&format!("[{}]", msg.role.prefix()), max_w));
        for raw_line in msg.text.lines() {
            push_wrapped(&mut wrapped, raw_line, max_w);
        }
        wrapped.push(String::new());
    }
    let end = wrapped.len().saturating_sub(dialog.scroll);
    let start = end.saturating_sub(max_h);
    wrapped.get(start..end).unwrap_or_default().to_vec()
}

fn push_wrapped(out: &mut Vec<String>, line: &str, max_w: usize) {
    if line.is_empty() {
        out.push(String::new());
        return;
    }
    let mut remaining = line;
    while !remaining.is_empty() {
        let split = remaining
            .char_indices()
            .nth(max_w)
            .map_or(remaining.len(), |(idx, _)| idx);
        let (head, tail) = remaining.split_at_checked(split).unwrap_or((remaining, ""));
        out.push(head.to_owned());
        remaining = tail;
    }
}
