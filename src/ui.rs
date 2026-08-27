//! Purpose: Renders the dual-panel terminal UI, status bars, and core layout.
//!
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph},
    Frame,
};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::console::ConsoleLine;
#[cfg(test)]
use crate::models::Viewer;
use crate::models::{App, CharColumn, CommandLine, Overlay, Panel, PanelSide, Recall, ViewMode};
use crate::scroll::ScrollBack;

/// Right-aligned width of the size column ("<DIR>", "1.50 MB", "4.00 KB").
const SIZE_FIELD_WIDTH: Cells = Cells(8);

/// Columns a panel row spends before the name: two borders, the two-cell
/// highlight symbol ("» "), and the two-cell icon plus its trailing space.
const PANEL_ROW_CHROME_WIDTH: Cells = Cells(2 + 2 + 3);

/// Narrowest name column the panel will render.
///
/// Without a floor, the fixed-width size column has absolute priority and a
/// narrow terminal (30 columns → 15 per panel) leaves the name exactly zero
/// cells, so `fit_name` returns "" for every entry and the panel becomes a
/// column of icons and sizes with no filenames at all. A file manager that
/// cannot show file names is useless, so the size column yields instead.
const MIN_NAME_WIDTH: Cells = Cells(8);

/// Separates the command line's path from the typed command.
const PROMPT_SUFFIX: &str = "> ";

mod overlays;

use overlays::render_overlays;

/// Dark blue background color, matching the classic Far Manager look
pub(crate) const FAR_BLUE: Color = Color::Rgb(0, 0, 128);

/// Draw the entire user interface: header, two file panels, status bar, and overlays.
pub fn ui(f: &mut Frame, app: &mut App) {
    // The console transcript is the bottom layer of every frame, exactly as
    // Far Manager keeps command output on the console screen and draws the
    // panels over it. Painting it first — rather than only in Ctrl+O mode — is
    // what makes "behind the panels" true rather than a figure of speech:
    // whatever the panels do not cover shows the console through.
    let area = f.area();
    render_console_backdrop(f, area);

    // Ctrl+O lifts the panels off, leaving that layer on screen. The bottom row
    // belongs to the prompt (or, with no prompt open, the hint), so the
    // transcript is drawn above it — writing into the full area first would put
    // the newest line under that row, which is precisely the line the user
    // pressed Ctrl+O to read.
    if app.view == ViewMode::Console {
        let bottom = Rect {
            y: area.y + area.height.saturating_sub(1),
            height: 1,
            ..area
        };
        render_console_text(
            f,
            app,
            Rect {
                height: area.height.saturating_sub(1),
                ..area
            },
        );
        if matches!(app.overlay, Overlay::CommandLine(_)) {
            // Far's Ctrl+O keeps the prompt usable, with each command's output
            // appearing directly above it. That is why `ViewMode` is separate
            // from `Overlay`: as one enum these two could not coexist.
            render_command_line(f, app, bottom);
            render_completion_menu(f, app, bottom);
        } else {
            render_console_hint(f, app, area);
        }
        render_overlays(f, app);
        return;
    }
    render_console_text(f, app, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(HEADER_HEIGHT),
            Constraint::Min(0), // panels — all remaining space
            Constraint::Length(STATUS_HEIGHT),
            Constraint::Length(COMMAND_LINE_HEIGHT),
            Constraint::Length(FKEY_BAR_HEIGHT),
        ])
        .split(f.area());

    let panels = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[1]);

    render_header(f, chunks[0]);

    render_panel(
        f,
        &mut app.left_panel,
        panels[0],
        app.active_panel == PanelSide::Left,
    );

    render_panel(
        f,
        &mut app.right_panel,
        panels[1],
        app.active_panel == PanelSide::Right,
    );

    render_status(f, app, chunks[2]);
    render_command_line(f, app, chunks[3]);
    render_completion_menu(f, app, chunks[3]);
    render_command_bar(f, chunks[4]);
    render_overlays(f, app);
}

/// Screen layout (top to bottom): header, panels (all remaining space), status
/// bar, command line, F-key bar. Named so [`panel_page_size`] stays in step with
/// the constraints used in [`ui`].
const HEADER_HEIGHT: u16 = 1;
const STATUS_HEIGHT: u16 = 1;
const COMMAND_LINE_HEIGHT: u16 = 1;
const FKEY_BAR_HEIGHT: u16 = 1;

/// Total non-panel lines claimed by the vertical layout above.
const CHROME_HEIGHT: u16 = HEADER_HEIGHT + STATUS_HEIGHT + COMMAND_LINE_HEIGHT + FKEY_BAR_HEIGHT;

/// Top and bottom border rows of a bordered widget (file panels, the
/// completion menu popup), which hold no content of their own.
const PANEL_BORDERS: u16 = 2;

/// Most candidate rows the completion dropdown shows at once.
const MENU_MAX_ROWS: usize = 10;

/// Screen height assumed when the backend cannot report its size (a detached or
/// non-tty terminal). The classic 24-line screen keeps paging usable instead of
/// degrading to a one-row step.
pub const FALLBACK_TERMINAL_HEIGHT: u16 = 24;

/// Entry rows visible in one file panel — the distance PgUp/PgDn move.
///
/// Derived from `terminal_height` less the surrounding chrome. Never returns 0,
/// since a zero-size page would make paging silently do nothing on a very short
/// terminal.
#[must_use]
pub fn panel_page_size(terminal_height: u16) -> usize {
    let rows = terminal_height.saturating_sub(CHROME_HEIGHT + PANEL_BORDERS);
    usize::from(rows).max(1)
}

/// A width in terminal **cells** — the unit ratatui spends a row's budget in
/// (`Buffer::set_line` subtracts each grapheme's `cell_width`).
///
/// A newtype because this module measures three different things in `usize`:
/// bytes (for slicing), characters (the editor's cursor column) and screen
/// columns. They coincide only for ASCII, and the comments scattered through
/// this file record what happens when they are mixed up — a CJK glyph is one
/// char and two columns, so a char-budgeted span overruns its allotment and
/// pushes whatever follows off the row. `Cells` makes that a type error.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub(crate) struct Cells(usize);

impl Cells {
    pub(crate) const ZERO: Self = Self(0);

    /// The width `text` occupies on screen.
    pub(crate) fn of(text: &str) -> Self {
        Self(UnicodeWidthStr::width(text))
    }

    /// The width of one character. Control characters report no width and are
    /// treated as zero-cell.
    fn of_char(ch: char) -> Self {
        Self(UnicodeWidthChar::width(ch).unwrap_or(0))
    }

    /// A terminal rectangle's width, as a budget to spend.
    pub(crate) const fn columns(count: u16) -> Self {
        Self(count as usize)
    }

    /// A width already counted in cells — for a budget being shared out, where
    /// the ratios are plain arithmetic and only the result is a width again.
    pub(crate) const fn new(count: usize) -> Self {
        Self(count)
    }

    pub(crate) const fn get(self) -> usize {
        self.0
    }

    pub(crate) const fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub(crate) const fn saturating_sub(self, other: Self) -> Self {
        Self(self.0.saturating_sub(other.0))
    }
}

impl std::fmt::Display for Cells {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Add for Cells {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self(self.0 + other.0)
    }
}

impl std::iter::Sum for Cells {
    fn sum<I: Iterator<Item = Self>>(iter: I) -> Self {
        Self(iter.map(Self::get).sum())
    }
}

/// Longest prefix of `text` that fits in `width` cells, and the cells it uses.
///
/// A double-width glyph is dropped whole rather than half-drawn, so the result
/// can come up one cell short of `width`; callers pad from the returned count,
/// never from a char count.
fn take_cells(text: &str, width: Cells) -> (String, Cells) {
    let mut taken = String::new();
    let mut used = Cells::ZERO;
    for ch in text.chars() {
        let ch_cells = Cells::of_char(ch);
        if used + ch_cells > width {
            break;
        }
        taken.push(ch);
        used = used + ch_cells;
    }
    (taken, used)
}

/// `text` with its leading `width` cells dropped, and the cells actually
/// dropped — the horizontal-scroll twin of [`take_cells`].
///
/// A double-width glyph straddling the cut is dropped whole rather than
/// half-drawn, so the second value can exceed `width` by one; callers position
/// against it, never against `width`.
fn skip_cells(text: &str, width: Cells) -> (&str, Cells) {
    let mut used = Cells::ZERO;
    for (index, ch) in text.char_indices() {
        if used >= width {
            return (&text[index..], used);
        }
        used = used + Cells::of_char(ch);
    }
    ("", used)
}

/// Cells occupied by the first `column` characters of `text`.
///
/// Bridges the editor's char-indexed caret (right for editing) to the screen
/// column it actually lands on (right for drawing) — the one place the two
/// units legitimately meet, which is why both the conversion and its input
/// have names.
pub(crate) fn cells_before(text: &str, column: CharColumn) -> Cells {
    text.chars().take(column.chars()).map(Cells::of_char).sum()
}

/// First `width` cells of `text`, marked with a trailing ellipsis when cut.
fn head_cells(text: &str, width: Cells) -> String {
    if Cells::of(text) <= width {
        return text.to_string();
    }
    if width.is_zero() {
        return String::new();
    }
    let (mut cut, _) = take_cells(text, width.saturating_sub(Cells(1)));
    cut.push('…');
    cut
}

/// Fit `name` into exactly `width` columns: pad short names, cut long ones with
/// a trailing ellipsis so the size column beside them stays readable.
fn fit_name(name: &str, width: Cells) -> String {
    let fitted = head_cells(name, width);
    // Pad by cells, not chars: `{:<width$}` counts chars and under-pads a name
    // holding double-width glyphs, which misaligns the size column.
    let padding = " ".repeat(width.saturating_sub(Cells::of(&fitted)).get());
    format!("{fitted}{padding}")
}

/// Last `width` cells of `text`, marked with a leading ellipsis when cut.
/// Paths are cut at the head: the tail names the current directory.
fn tail_cells(text: &str, width: Cells) -> String {
    if Cells::of(text) <= width {
        return text.to_string();
    }
    if width.is_zero() {
        return String::new();
    }
    // Walk backwards so the tail — the part that identifies the directory —
    // is what survives, keeping only as much as fits beside the ellipsis.
    let mut kept: Vec<char> = Vec::new();
    let mut used = Cells(1); // the leading '…'
    for ch in text.chars().rev() {
        let ch_cells = Cells::of_char(ch);
        if used + ch_cells > width {
            break;
        }
        kept.push(ch);
        used = used + ch_cells;
    }
    let mut cut = String::from("…");
    cut.extend(kept.into_iter().rev());
    cut
}

fn render_panel(f: &mut Frame, panel: &mut Panel, area: Rect, is_active: bool) {
    let border_style = if is_active {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Blue)
    };

    let title = format!(" {} ", panel.path.display());

    // The name column gets whatever the panel is not already spending on chrome
    // and the size column — a fixed width would swallow the size on an
    // 80-column terminal, where each panel is only 40 wide. Below
    // MIN_NAME_WIDTH the priority flips and the size column is the one that
    // gets clipped; see MIN_NAME_WIDTH.
    let name_width = Cells::columns(area.width)
        .saturating_sub(PANEL_ROW_CHROME_WIDTH + SIZE_FIELD_WIDTH)
        .max(MIN_NAME_WIDTH);

    let items: Vec<ListItem> = panel
        .entries
        .iter()
        .map(|entry| {
            let is_marked = panel.marked.contains(entry.name.as_str());
            let icon = if entry.kind.is_dir() { "📁" } else { "📄" };
            let size_str: std::borrow::Cow<'static, str> = if entry.kind.is_dir() {
                "<DIR>".into()
            } else {
                entry.size.to_string().into()
            };

            let name_style = if is_marked {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else if entry.kind.is_dir() {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            };

            let size_style = if is_marked {
                Style::default().fg(Color::Yellow)
            } else {
                Style::default().fg(Color::LightCyan)
            };

            let line = Line::from(vec![
                Span::raw(icon),
                Span::raw(" "),
                Span::styled(fit_name(&entry.name, name_width), name_style),
                // The width is a `Cells` constant, and the format spec needs the
                // raw count: a size string is ASCII digits and a unit, so the
                // two agree here by construction.
                Span::styled(
                    format!("{size_str:>width$}", width = SIZE_FIELD_WIDTH.get()),
                    size_style,
                ),
            ]);

            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(border_style)
                .style(Style::default().bg(FAR_BLUE))
                .title(title)
                .title_style(Style::default().fg(Color::White).bg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .bg(Color::Cyan)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("» ");

    f.render_stateful_widget(list, area, panel.list_state_mut());
}

fn render_header(f: &mut Frame, area: Rect) {
    let paragraph = Paragraph::new("Far Manager style | Tab: Switch panels | F1 for help").style(
        Style::default()
            .fg(Color::Cyan)
            .bg(FAR_BLUE)
            .add_modifier(Modifier::BOLD),
    );
    f.render_widget(paragraph, area);
}

fn render_status(f: &mut Frame, app: &App, area: Rect) {
    let panel = match app.active_panel {
        PanelSide::Left => &app.left_panel,
        PanelSide::Right => &app.right_panel,
    };

    let selected_info = if let Some(entry) = panel.get_selected_entry() {
        let size_info = if entry.kind.is_dir() {
            "DIR".to_string()
        } else {
            entry.size.to_string()
        };
        format!("{} | {}", entry.name, size_info)
    } else {
        "No selection".to_string()
    };

    // Show quick search if active, otherwise show normal status
    let width = Cells::columns(area.width);
    let status_text = if let Some(search) = app.quick_search.query() {
        format!(" Quick search: {search}_")
    } else {
        // One unwrapped row, three tiers, each reserving its columns before
        // the next is even measured. Built the other way round — which is how
        // this read until it was fixed — the 69-column hint prefix plus the
        // selected-entry info filled an 80-column row exactly, so the message
        // was clipped off-screen and lost.
        //
        // 1. The message: the ONLY channel a failed copy, a refused view or a
        //    save error has. No `log_error` call exists in the handlers, so
        //    text clipped here is text the user never sees.
        // 2. The selected entry: cheap, and it changes with every keypress.
        // 3. The navigation hints: fixed text, learned once, and already
        //    echoed by the header and the F-key bar — so they yield first.
        let message = head_cells(&app.status_text(), width);
        let entry = head_cells(
            &format!("{selected_info} | "),
            width.saturating_sub(Cells::of(&message)),
        );
        let hint = head_cells(
            " ↑↓: Navigate | Enter: Open | Tab: Switch Panel | ←→: Select Panel | ",
            width.saturating_sub(Cells::of(&message) + Cells::of(&entry)),
        );
        format!("{hint}{entry}{message}")
    };

    let paragraph = Paragraph::new(status_text)
        .style(Style::default().bg(FAR_BLUE).fg(Color::Cyan))
        .block(Block::default().borders(Borders::NONE));

    f.render_widget(paragraph, area);
}

/// Paint the black ground the whole frame is drawn onto.
///
/// Separate from the text because the two cover different rectangles in Ctrl+O
/// mode, and because the ground is needed even for an empty log — a
/// transparent one would let the terminal's own colours show through the gaps.
fn render_console_backdrop(f: &mut Frame, area: Rect) {
    f.render_widget(
        Block::default().style(Style::default().bg(Color::Black)),
        area,
    );
}

/// Draw the console transcript into `area`, oldest visible line at the top.
///
/// One row per line, filling downwards from the top like the console screen
/// this stands in for. Command echoes are bright so a long transcript reads as
/// a sequence of commands rather than an undifferentiated wall; stderr is red.
fn render_console_text(f: &mut Frame, app: &App, area: Rect) {
    let height = usize::from(area.height);
    let width = Cells::columns(area.width);
    let lines: Vec<Line> = app
        .console
        .visible(height)
        .into_iter()
        .map(|line| {
            let style = match line {
                ConsoleLine::Command(_) => Style::default()
                    .fg(Color::White)
                    .bg(Color::Black)
                    .add_modifier(Modifier::BOLD),
                ConsoleLine::Output(_) => Style::default().fg(Color::Gray).bg(Color::Black),
                ConsoleLine::Error(_) => Style::default().fg(Color::LightRed).bg(Color::Black),
            };
            // Cut rather than wrap: one transcript line per row keeps the
            // scroll arithmetic in `ConsoleLog` honest — a wrapped line would
            // occupy rows the log does not know it spent.
            Line::from(Span::styled(head_cells(line.text(), width), style))
        })
        .collect();

    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(Color::Black)),
        area,
    );
}

/// The bottom-row hint shown while Ctrl+O holds the panels off screen.
fn render_console_hint(f: &mut Frame, app: &App, area: Rect) {
    let hint = if app.console.is_empty() {
        "Console is empty — run a command from the command line. Ctrl+O returns".to_owned()
    } else if app.console.scroll() == ScrollBack::BOTTOM {
        "Ctrl+O returns  |  ↑↓ PgUp/PgDn Home/End scroll".to_owned()
    } else {
        format!(
            "Ctrl+O returns  |  ↑↓ PgUp/PgDn Home/End scroll  |  {} line(s) below",
            app.console.scroll().lines()
        )
    };
    let row = Rect {
        x: area.x,
        y: area.y + area.height.saturating_sub(1),
        width: area.width,
        height: 1,
    };
    f.render_widget(Clear, row);
    f.render_widget(
        Paragraph::new(head_cells(&hint, Cells::columns(row.width)))
            .style(Style::default().fg(Color::DarkGray).bg(Color::Black)),
        row,
    );
}

/// Render the command line — always visible like in Far Manager.
///
/// Shows `path>` and, while the prompt is open, the typed text with the caret
/// rendered as a reversed cell **where the caret actually is**. A trailing
/// glyph would be a lie now that the caret can sit mid-line, and a caret the
/// user cannot see is the same as no caret at all.
fn render_command_line(f: &mut Frame, app: &App, area: Rect) {
    let state = match &app.overlay {
        Overlay::CommandLine(state) => Some(state),
        _ => None,
    };

    let prompt_text = command_prompt(app, state);
    let width = Cells::columns(area.width);

    let Some(state) = state else {
        // Prompt closed: the path and its suffix, windowed to the row. The row
        // stays drawn — Far Manager's command line is always visible, whether
        // or not it is taking input.
        let prompt = format!(
            "{}{PROMPT_SUFFIX}",
            tail_cells(&prompt_text, width.saturating_sub(Cells::of(PROMPT_SUFFIX)))
        );
        render_command_row(f, area, &prompt, CommandRow::default(), width);
        return;
    };

    let before = state.line.before_cursor();
    let after = state.line.after_cursor();
    // Ghost text is only ever offered with the caret at the end (see
    // `popup::command_line::refresh_ghost`), so it can never collide with
    // `after` — but read it defensively rather than assuming.
    let ghost = if state.line.at_end() {
        state.ghost.as_deref().unwrap_or("")
    } else {
        ""
    };

    // The caret shows the character it sits on; at the end of the line it
    // borrows the first ghost character, so the suggestion reads as one
    // continuous word instead of being pushed a cell to the right.
    let caret_char = after
        .chars()
        .next()
        .or_else(|| ghost.chars().next())
        .unwrap_or(' ');
    let caret: String = caret_char.into();
    // Whichever run the caret consumed a character from continues after it.
    let (rest, ghost_rest) = if after.is_empty() {
        ("", ghost.get(caret_char.len_utf8()..).unwrap_or(""))
    } else {
        (after.get(caret_char.len_utf8()..).unwrap_or(""), ghost)
    };

    // The typed text is windowed FIRST, against everything but the prompt and
    // the caret. It is the one part of this row the user cannot read anywhere
    // else. The pre-caret text keeps its *tail* so the caret stays on screen
    // however long the line grows — a window anchored at the start would let
    // the user type blind past the terminal width.
    let budget = width.saturating_sub(Cells::of(&caret) + Cells::of(PROMPT_SUFFIX));
    let head = tail_cells(before, budget);

    // The path is next in line and is the part that can grow without bound, so
    // it is what gets cut before the tail or the ghost is even considered.
    let reserved = Cells::of(&head)
        + Cells::of(&caret)
        + Cells::of(rest)
        + Cells::of(ghost_rest)
        + Cells::of(PROMPT_SUFFIX);
    let prompt = format!(
        "{}{PROMPT_SUFFIX}",
        tail_cells(&prompt_text, width.saturating_sub(reserved))
    );

    // Clamp what follows the caret to the room actually left: the path may not
    // have shrunk by the full amount requested. The tail yields before the
    // ghost does, and the typed text before the caret never yields at all.
    let used = Cells::of(&prompt) + Cells::of(&head) + Cells::of(&caret);
    let (rest_shown, rest_cells) = take_cells(rest, width.saturating_sub(used));
    let (ghost_shown, _) = take_cells(ghost_rest, width.saturating_sub(used + rest_cells));

    render_command_row(
        f,
        area,
        &prompt,
        CommandRow {
            head,
            caret,
            rest: rest_shown,
            ghost: ghost_shown,
        },
        width,
    );
}

/// The text before `PROMPT_SUFFIX`: normally the active panel's path, but the
/// bash reverse-search banner while `Ctrl+R` is running, so the user can see
/// what is being matched.
fn command_prompt(app: &App, state: Option<&CommandLine>) -> String {
    if let Some(CommandLine {
        recall: Recall::Searching { query, .. },
        ..
    }) = state
    {
        return format!("(reverse-i-search)`{query}'");
    }
    match app.active_panel {
        PanelSide::Left => app.left_panel.path.display().to_string(),
        PanelSide::Right => app.right_panel.path.display().to_string(),
    }
}

/// The four already-windowed runs that make up an open command line.
#[derive(Default)]
struct CommandRow {
    /// Typed text before the caret.
    head: String,
    /// The single cell under the caret; empty while the prompt is closed.
    caret: String,
    /// Typed text after the caret.
    rest: String,
    /// History suggestion beyond the caret.
    ghost: String,
}

/// Paint one command-line row from its pre-windowed runs.
fn render_command_row(f: &mut Frame, area: Rect, prompt: &str, row: CommandRow, width: Cells) {
    let used = Cells::of(prompt)
        + Cells::of(&row.head)
        + Cells::of(&row.caret)
        + Cells::of(&row.rest)
        + Cells::of(&row.ghost);
    let padding = " ".repeat(width.saturating_sub(used).get());

    let typed = Style::default().fg(Color::White).bg(Color::Black);
    let spans = vec![
        Span::styled(
            prompt.to_owned(),
            Style::default().fg(Color::LightCyan).bg(Color::Black),
        ),
        Span::styled(row.head, typed),
        // A reversed cell, not a "_" glyph: the caret has to be legible when it
        // sits *on* a character rather than after the last one.
        Span::styled(
            row.caret,
            Style::default().fg(Color::Black).bg(Color::Yellow),
        ),
        Span::styled(row.rest, typed),
        Span::styled(
            row.ghost,
            Style::default().fg(Color::DarkGray).bg(Color::Black),
        ),
        Span::styled(padding, Style::default().bg(Color::Black)),
    ];

    let paragraph = Paragraph::new(Line::from(spans)).style(Style::default().bg(Color::Black));
    f.render_widget(paragraph, area);
}

/// Draw the completion dropdown directly above the command line.
///
/// The window scrolls to keep the highlighted candidate visible, so a long
/// candidate list stays navigable without growing the popup.
fn render_completion_menu(f: &mut Frame, app: &App, command_line_area: Rect) {
    let Overlay::CommandLine(state) = &app.overlay else {
        return;
    };
    let Some(menu) = &state.menu else {
        return;
    };

    // Row budget must come from the space actually available above the
    // command line, not from the wish-list MENU_MAX_ROWS — otherwise the
    // scroll window below is sized too large, `render_widget` (which draws
    // with a fresh, unselected `ListState`) truncates from the end of the
    // item slice to fit the clamped area, and that truncation can cut off
    // exactly the row carrying `selected`.
    let available = usize::from(
        command_line_area
            .y
            .saturating_sub(PANEL_BORDERS.min(command_line_area.y)),
    );
    let visible = menu.candidates().len().min(MENU_MAX_ROWS).min(available);
    if visible == 0 {
        // No room for even one candidate row — an empty bordered box would
        // just be noise.
        return;
    }
    // Scroll so `selected` sits inside the window.
    let first = menu.selected().saturating_sub(visible - 1);

    let height = u16::try_from(visible).unwrap_or(u16::MAX) + PANEL_BORDERS;
    debug_assert!(height <= command_line_area.y);
    let area = Rect {
        x: command_line_area.x,
        y: command_line_area.y.saturating_sub(height),
        width: command_line_area.width,
        height,
    };

    let inner_width = Cells::columns(area.width.saturating_sub(PANEL_BORDERS));
    let items: Vec<ListItem> = menu
        .candidates()
        .iter()
        .enumerate()
        .skip(first)
        .take(visible)
        .map(|(index, candidate)| {
            let hint = candidate.hint.as_deref().unwrap_or("");
            // Cells, not chars — same rule as `fit_name`: a char count
            // over-fills the row for double-width glyphs and pushes the
            // right-aligned hint off the popup.
            let gap = inner_width.saturating_sub(Cells::of(&candidate.text) + Cells::of(hint));
            let style = if index == menu.selected() {
                Style::default()
                    .bg(Color::Cyan)
                    .fg(Color::Black)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Cyan)
            };
            ListItem::new(Line::from(Span::styled(
                format!("{}{}{hint}", candidate.text, " ".repeat(gap.get())),
                style,
            )))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(FAR_BLUE)),
    );

    f.render_widget(Clear, area);
    f.render_widget(list, area);
}

fn render_command_bar(f: &mut Frame, area: Rect) {
    let keys = [
        ("1", "Help"),
        ("2", "UserMn"),
        ("3", "View"),
        ("4", "Edit"),
        ("5", "Copy"),
        ("6", "RenMov"),
        ("7", "MkFold"),
        ("8", "Delete"),
        ("9", "MCP"),
        ("10", "Quit"),
    ];

    let spans: Vec<Span> = keys
        .iter()
        .flat_map(|(key, label)| {
            vec![
                Span::styled(*key, Style::default().bg(Color::Black).fg(Color::Cyan)),
                Span::styled(
                    format!("{label:<6}"),
                    Style::default().bg(Color::Cyan).fg(Color::Black),
                ),
            ]
        })
        .collect();

    let paragraph = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(Color::Cyan).fg(Color::Black))
        .block(Block::default().borders(Borders::NONE));

    f.render_widget(paragraph, area);
}

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let percent_y = percent_y.min(100);
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ]
            .as_ref(),
        )
        .split(r);

    let percent_x = percent_x.min(100);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints(
            [
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ]
            .as_ref(),
        )
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_console;
#[cfg(test)]
mod tests_delete_confirm_paths;
#[cfg(test)]
mod tests_editor_confirm_exit;
#[cfg(test)]
mod tests_overlay_dialogs;
#[cfg(test)]
mod tests_render_widths;
