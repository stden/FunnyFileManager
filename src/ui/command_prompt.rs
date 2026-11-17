//! Purpose: Builds and windows the command line's prompt segments and paints each in its part's colour.

use ratatui::{
    style::{Color, Modifier, Style},
    text::Span,
};

use crate::models::{App, CommandLine, PanelSide, Recall};

use super::{palette, tail_cells, Cells};

/// The text before `PROMPT_SUFFIX`: normally the active panel's path, but the
/// bash reverse-search banner while `Ctrl+R` is running, so the user can see
/// what is being matched.
pub(super) fn command_prompt(
    app: &App,
    state: Option<&CommandLine>,
) -> Vec<crate::prompt::Segment> {
    use crate::prompt::{LastCommand, Part, Segment};

    if let Some(CommandLine {
        recall: Recall::Searching { query, .. },
        ..
    }) = state
    {
        // A reverse-i-search replaces the whole prompt with its own text.
        return vec![Segment {
            text: format!("(reverse-i-search)`{query}'"),
            part: Part::Path,
        }];
    }
    let directory = match app.active_panel {
        PanelSide::Left => &app.left_panel.path,
        PanelSide::Right => &app.right_panel.path,
    };
    crate::prompt::segments(directory, LastCommand::from_status(&app.status))
}

/// Fit the prompt into `width`, cutting the path from its head first.
///
/// The path is the one part that grows without bound, and its *tail* is what
/// names the directory — so it yields before anything else, and from the left.
/// Only when it has nothing left to give do whole segments drop, front first:
/// the sigil and the branch are worth more per column than the login name.
pub(super) fn window_prompt(
    prompt: Vec<crate::prompt::Segment>,
    width: Cells,
) -> Vec<crate::prompt::Segment> {
    use crate::prompt::Part;

    let total = prompt_width(&prompt);
    if total <= width {
        return prompt;
    }

    let mut prompt = prompt;
    let overflow = total.saturating_sub(width);
    if let Some(path) = prompt.iter_mut().find(|s| s.part == Part::Path) {
        let room = Cells::of(&path.text).saturating_sub(overflow);
        path.text = tail_cells(&path.text, room);
    }

    while prompt_width(&prompt) > width && prompt.len() > 1 {
        prompt.remove(0);
    }
    prompt
}

/// The cells a prompt occupies, measured on the pieces that get drawn.
pub(super) fn prompt_width(prompt: &[crate::prompt::Segment]) -> Cells {
    prompt.iter().map(|segment| Cells::of(&segment.text)).sum()
}

/// Paint the prompt's segments, each in the colour its part calls for.
///
/// Built from [`crate::prompt::segments`] rather than parsed back out of a
/// string: the pieces already know what they are, and re-deriving that from
/// the text would make `root` in a directory name look like a login.
pub(super) fn prompt_spans<'a>(prompt: &[crate::prompt::Segment]) -> Vec<Span<'a>> {
    use crate::prompt::Part;

    prompt
        .iter()
        .cloned()
        .map(|segment| {
            let colour = match segment.part {
                Part::StatusOk => palette::PROMPT_OK,
                Part::StatusFailed => palette::PROMPT_FAILED,
                Part::User | Part::Host => palette::PROMPT_USER,
                Part::At => palette::PROMPT_AT,
                Part::Path => palette::PROMPT_PATH,
                Part::Branch => palette::PROMPT_BRANCH,
                Part::Sigil | Part::Space => palette::PROMPT_SIGIL,
            };
            let style = Style::default().fg(colour).bg(Color::Black);
            Span::styled(
                segment.text,
                if segment.part == Part::Path {
                    style.add_modifier(Modifier::BOLD)
                } else {
                    style
                },
            )
        })
        .collect()
}
