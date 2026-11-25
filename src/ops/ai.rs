//! Purpose: AI assistant entry points (`F9 → Commands`, `Alt+F1/F2`, `:ai`, `:ask`).

pub mod actions;

#[cfg(test)]
mod tests;

use crate::command::AiCommand;
use crate::models::{App, Overlay};
use crate::values::ai::AiProvider;

/// Open the Warp-like AI assistant overlay.
pub fn open(app: &mut App) {
    open_with(app, None, None);
}

/// Open the AI assistant overlay with an optional initial prompt and provider.
pub fn open_with(app: &mut App, prompt: Option<&str>, provider: Option<AiProvider>) {
    let dialog = actions::initial_dialog(app, prompt, provider);
    app.set_overlay(Overlay::Ai(Box::new(dialog)));
}

/// Dispatch `:ai` / `:ask` commands into opening, switching provider, or editing `ffm.yml`.
pub fn command(app: &mut App, cmd: AiCommand) -> std::io::Result<()> {
    match cmd {
        AiCommand::Open(prompt) => {
            open_with(app, prompt.as_deref(), None);
            Ok(())
        }
        AiCommand::Provider(provider) => {
            open_with(app, None, Some(provider));
            Ok(())
        }
        AiCommand::Edit => edit_config(app),
    }
}

/// Open `ffm.yml` in the built-in editor to configure `ai:`.
pub fn edit_config(app: &mut App) -> std::io::Result<()> {
    crate::ops::network_drives::edit_config(app)
}
