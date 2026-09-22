//! Purpose: Far's modal dialogs — the frame, the fields, the buttons, the shadow.
//!
//! One renderer for every modal, parameterised by the colour it stands on. Far
//! uses that colour to say what kind of question is being asked: red for
//! anything that can lose data, grey for anything that merely wants an answer.
//! The colour lands before the text is read, which for the destructive case is
//! exactly when a reflexive Enter would otherwise already have been pressed.
//!
//! The shape is Far's too: a double frame carrying the title, an inner
//! single-line box around the body, and `├───┤` rules dividing that box into
//! groups — the last of which is always the buttons.
//!
//! Sized to its content and centred, because a fixed-percentage box either
//! wraps a short question across a huge empty field or clips a long file name.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph},
    Frame,
};

use super::{far_block, palette, Cells};

/// Blank dialog surface around the frame, top/bottom and left/right.
///
/// Far insets the frame rather than running it to the edge, so the dialog
/// reads as a panel with a box on it rather than as a box with nothing around
/// it. Two columns to one row, because a cell is about twice as tall as wide.
const MARGIN_X: u16 = 2;
const MARGIN_Y: u16 = 1;

/// Rows the chrome spends: the margin twice, the frame twice, the rule above
/// the buttons, and the button row itself.
const CHROME_ROWS: u16 = MARGIN_Y * 2 + 2 + 1 + 1;

/// Columns the chrome spends: the margin twice, the frame twice, and one blank
/// inside the frame on each side.
const CHROME_COLUMNS: u16 = MARGIN_X * 2 + 2 + 2;

/// How far the drop shadow is offset. Two columns and one row is the ratio a
/// terminal cell has, so the shadow reads as square.
const SHADOW_X: u16 = 2;
const SHADOW_Y: u16 = 1;

/// Blank columns between two buttons.
const BUTTON_GAP: &str = " ";

/// Columns an input field claims regardless of what is in it.
const INPUT_WIDTH: usize = 34;

/// The character that marks the next letter as a control's hotkey.
///
/// Far's own convention. It is needed because the hotkey is not always the
/// first letter — `Fu&zzy`, `Find pre&vious` — so picking one positionally
/// would be wrong exactly where the user most needs it to be right.
const HOTKEY_MARK: char = '&';

/// What kind of question a dialog is asking, which is what its colour says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Ground {
    /// Something can be lost by answering wrongly.
    Warning,
    /// Something merely needs to be supplied.
    Prompt,
}

impl Ground {
    fn style(self) -> Style {
        match self {
            Self::Warning => Style::default()
                .bg(palette::WARNING_BG)
                .fg(palette::WARNING_FG),
            Self::Prompt => Style::default()
                .bg(palette::DIALOG_BG)
                .fg(palette::DIALOG_FG),
        }
    }
}

/// One button in a dialog.
pub(super) struct Button<'a> {
    /// May carry a [`HOTKEY_MARK`], which is stripped before drawing.
    pub label: &'a str,
    /// Drawn `{ like this }` rather than `[ like this ]`. Far marks the button
    /// Enter starts on, so the user can see what a reflex will do.
    pub is_default: bool,
    /// Carries the cursor: highlighted, and what Enter activates now.
    pub is_focused: bool,
    /// Drawn dimmed and skipped by the cursor. Far shows a button that does
    /// not apply rather than hiding it, so the dialog keeps its shape and the
    /// user learns where the control will be when it does apply.
    pub is_enabled: bool,
}

impl<'a> Button<'a> {
    /// A plain button: present, active, not the default.
    pub const fn new(label: &'a str) -> Self {
        Self {
            label,
            is_default: false,
            is_focused: false,
            is_enabled: true,
        }
    }

    pub const fn default_button(mut self) -> Self {
        self.is_default = true;
        self
    }

    pub const fn focused(mut self, focused: bool) -> Self {
        self.is_focused = focused;
        self
    }

    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.is_enabled = enabled;
        self
    }
}

impl Button<'_> {
    /// The width this button costs on the row, measured where it is drawn so
    /// the sizing and the drawing cannot disagree.
    fn width(&self) -> Cells {
        // Two brackets and two blanks, plus the label without its marker.
        Cells::of(&self.label.replace(HOTKEY_MARK, "")) + Cells::new(4)
    }
}

/// One line of a dialog's body.
pub(super) enum Row<'a> {
    /// A sentence, centred — what a warning is made of.
    Text(&'a str),
    /// A field's caption, left-aligned above it.
    Label(&'a str),
    /// A text field, sunk into the dialog on its own ground.
    ///
    /// `history` draws Far's `↓` marker at the right edge, which is how a
    /// field says it remembers what was typed into it before.
    Input {
        value: &'a str,
        focused: bool,
        history: bool,
    },
    /// A checkbox, drawn as Far draws it: `[x] Label`.
    ///
    /// `enabled` is drawn dimmed and skipped by the cursor, for an option that
    /// does not apply to this particular dialog.
    Checkbox {
        label: &'a str,
        checked: bool,
        focused: bool,
        enabled: bool,
    },
    /// A row of mutually exclusive choices: `Caption: (•) One ( ) Two`.
    ///
    /// All on one line, as Far lays them out. Three short options read faster
    /// side by side than stacked, and the dialog is already tall.
    Radio {
        caption: &'a str,
        options: &'a [&'a str],
        selected: usize,
        focused: bool,
    },
    /// A caption and a closed drop-down: `Caption: [value]↓`.
    Choice {
        caption: &'a str,
        value: &'a str,
        focused: bool,
    },
    /// A caption and a text field on the same line, drawn like a [`Row::Choice`]
    /// so that a group of the two lines its boxes up.
    ///
    /// `enabled` is dimmed and skipped by the cursor, for a field that does not
    /// apply to the answer currently given above it.
    Field {
        caption: &'a str,
        value: &'a str,
        focused: bool,
        enabled: bool,
    },
    /// A rule across the inner box, dividing one group of controls from the
    /// next.
    Divider,
}

impl<'a> Row<'a> {
    pub const fn input(value: &'a str, focused: bool, history: bool) -> Self {
        Self::Input {
            value,
            focused,
            history,
        }
    }

    pub const fn checkbox(label: &'a str, checked: bool, focused: bool) -> Self {
        Self::Checkbox {
            label,
            checked,
            focused,
            enabled: true,
        }
    }
}

/// The least room a captioned control gives its value, before the `↓`.
const CHOICE_WIDTH: usize = 20;

/// Columns a caption claims before its value box begins.
///
/// A minimum rather than a fixed width: short captions are padded so the boxes
/// under one another start in the same column, which is how Far lays a group
/// out, and a caption longer than this simply takes the room it needs.
const CAPTION_WIDTH: usize = 12;

/// Blanks between a caption and its box — at least one, and enough to reach
/// [`CAPTION_WIDTH`].
fn caption_gap(caption: &str) -> usize {
    let plain = Cells::of(&caption.replace(HOTKEY_MARK, "")).get();
    CAPTION_WIDTH.max(plain + 1) - plain
}

/// The marker Far puts at the right of a field that has a history or a list.
const DROP_MARKER: char = '↓';

/// How one radio option is drawn when it is, and is not, the chosen one.
const RADIO_ON: &str = "(•) ";
const RADIO_OFF: &str = "( ) ";

impl Row<'_> {
    /// The columns this row needs, so the frame can be sized before it is drawn.
    fn width(&self) -> Cells {
        let plain = |text: &str| Cells::of(&text.replace(HOTKEY_MARK, ""));
        match self {
            Self::Text(text) | Self::Label(text) => plain(text),
            // The field is given room to grow rather than sized to what has
            // been typed, or the dialog would resize on every keystroke.
            Self::Input { value, .. } => Cells::of(value).max(Cells::new(INPUT_WIDTH)),
            Self::Checkbox { label, .. } => plain(label) + Cells::new(4),
            Self::Radio {
                caption, options, ..
            } => {
                let choices: Cells = options
                    .iter()
                    .map(|option| plain(option) + Cells::of(RADIO_ON))
                    .sum();
                plain(caption) + Cells::new(1) + choices
            }
            Self::Choice { caption, .. } | Self::Field { caption, .. } => {
                plain(caption) + Cells::new(caption_gap(caption) + CHOICE_WIDTH + 1)
            }
            Self::Divider => Cells::ZERO,
        }
    }
}

/// Where a dialog and its parts sit on screen.
///
/// Computed from the screen size and the dialog's own content, and by exactly
/// one function — [`place`] — which both the renderer and the mouse call. A
/// click is only correct if the arithmetic that decides what was hit is the
/// arithmetic that decided where to draw it; two copies would drift, and a
/// drifted copy sends the click to the wrong button rather than to nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Placement {
    /// The whole dialog, shadow excluded.
    pub area: Rect,
    /// Inside the frame: the body rows, then the rule, then the buttons.
    pub body: Rect,
    /// The row the buttons are drawn on.
    pub buttons_y: u16,
}

/// Size and centre a dialog for `screen`.
///
/// `None` when the screen cannot hold even the frame, which is the same
/// condition [`render`] gives up on.
pub(super) fn place(
    screen: Rect,
    title: &str,
    rows: &[Row],
    buttons: &[Button],
    offset: (i32, i32),
) -> Option<Placement> {
    let (width, height) = dialog_size(screen, title, rows, buttons);
    // Centred, then shifted by however far the user has dragged it. The shift
    // is clamped so a dialog can never be dragged off the screen and lost.
    let centred_x = i32::from(screen.x) + i32::from(screen.width.saturating_sub(width)) / 2;
    let centred_y = i32::from(screen.y) + i32::from(screen.height.saturating_sub(height)) / 2;
    let max_x = i32::from(screen.right().saturating_sub(width));
    let max_y = i32::from(screen.bottom().saturating_sub(height));
    let area = Rect {
        x: u16::try_from((centred_x + offset.0).clamp(i32::from(screen.x), max_x.max(0))).ok()?,
        y: u16::try_from((centred_y + offset.1).clamp(i32::from(screen.y), max_y.max(0))).ok()?,
        width,
        height,
    };

    let framed = frame_area(area);
    if framed.height < 3 || framed.width < 4 {
        return None;
    }
    let body = Rect {
        x: framed.x + 1,
        y: framed.y + 1,
        width: framed.width.saturating_sub(2),
        height: framed.height.saturating_sub(2),
    };
    Some(Placement {
        area,
        body,
        buttons_y: body.y + body.height.saturating_sub(1),
    })
}

fn dialog_size(screen: Rect, title: &str, rows: &[Row], buttons: &[Button]) -> (u16, u16) {
    let button_row: Cells = buttons
        .iter()
        .map(|button| button.width() + Cells::of(BUTTON_GAP))
        .sum();

    let widest = rows
        .iter()
        .map(Row::width)
        .chain(std::iter::once(button_row))
        .chain(std::iter::once(Cells::of(title) + Cells::new(4)))
        .max()
        .unwrap_or_default();

    let width = u16::try_from(widest.get())
        .unwrap_or(u16::MAX)
        .saturating_add(CHROME_COLUMNS)
        .min(screen.width);
    let height = u16::try_from(rows.len())
        .unwrap_or(u16::MAX)
        .saturating_add(CHROME_ROWS)
        .min(screen.height);

    (width, height)
}

fn frame_area(area: Rect) -> Rect {
    Rect {
        x: area.x + MARGIN_X,
        y: area.y + MARGIN_Y,
        width: area.width.saturating_sub(MARGIN_X * 2),
        height: area.height.saturating_sub(MARGIN_Y * 2),
    }
}

/// The screen columns each button covers, in the order they were given.
///
/// The same centring `button_line` draws with, so a click lands on the button
/// the user is looking at.
pub(super) fn button_spans(placement: &Placement, buttons: &[Button]) -> Vec<(usize, u16, u16)> {
    let total: usize = buttons
        .iter()
        .map(|button| button.width().get() + Cells::of(BUTTON_GAP).get())
        .sum::<usize>()
        .saturating_sub(Cells::of(BUTTON_GAP).get());

    let width = usize::from(placement.body.width);
    let mut left = placement.body.x + u16::try_from(width.saturating_sub(total) / 2).unwrap_or(0);
    let mut spans = Vec::with_capacity(buttons.len());
    for (index, button) in buttons.iter().enumerate() {
        let cells = u16::try_from(button.width().get()).unwrap_or(0);
        spans.push((index, left, left.saturating_add(cells)));
        left = left
            .saturating_add(cells)
            .saturating_add(u16::try_from(Cells::of(BUTTON_GAP).get()).unwrap_or(0));
    }
    spans
}

/// Draw a dialog and record where its parts landed, for the next click.
///
/// The recording is the point: a dialog is sized to its own content, so unlike
/// the panels its geometry cannot be recomputed from the screen size alone.
/// The loop draws immediately before it reads an event, so what is recorded
/// here is exactly the frame the user was looking at.
pub(super) fn render_tracked(
    f: &mut Frame,
    tracker: &mut crate::mouse::DialogMouse,
    title: &str,
    ground: Ground,
    rows: &[Row],
    buttons: &[Button],
) -> Option<(u16, u16)> {
    let offset = tracker.offset;
    let caret = render_at(f, title, ground, rows, buttons, offset);
    if let Some(placement) = place(f.area(), title, rows, buttons, offset) {
        tracker.area = Some(placement.area);
        tracker.body = Some(placement.body);
        tracker.buttons_y = placement.buttons_y;
        tracker.buttons = button_spans(&placement, buttons);
    } else {
        tracker.clear();
    }
    caret
}

/// The same dialog, shifted by however far it has been dragged.
pub(super) fn render_at(
    f: &mut Frame,
    title: &str,
    ground: Ground,
    rows: &[Row],
    buttons: &[Button],
    offset: (i32, i32),
) -> Option<(u16, u16)> {
    let screen = f.area();
    let placement = place(screen, title, rows, buttons, offset)?;
    let area = placement.area;

    render_shadow(f, area, screen);

    let style = ground.style();
    // The surface first, then the frame inset into it.
    f.render_widget(Clear, area);
    f.render_widget(Paragraph::new("").style(style), area);

    let framed = frame_area(area);
    let frame = far_block()
        .title(Line::from(Span::styled(format!(" {title} "), style)).centered())
        .border_style(style)
        .style(style);
    let body = placement.body;
    f.render_widget(frame, framed);

    let caret = render_body(f, rows, style, body, framed);

    render_rule(f, style, body.y + body.height.saturating_sub(2), framed);
    f.render_widget(
        Paragraph::new(button_line(buttons, style)).style(style),
        Rect {
            y: body.y + body.height - 1,
            height: 1,
            ..body
        },
    );
    caret
}

fn render_body(
    f: &mut Frame,
    rows: &[Row],
    style: Style,
    body: Rect,
    framed: Rect,
) -> Option<(u16, u16)> {
    let mut caret = None;
    for (index, row) in rows.iter().enumerate() {
        let Ok(offset) = u16::try_from(index) else {
            break;
        };
        if offset >= body.height.saturating_sub(2) {
            break;
        }
        let y = body.y + offset;
        // One past the last character: the caret sits where the next one will
        // go, which is where a text field's cursor belongs.
        let column = row_caret(row);
        if let Some(column) = column {
            caret = Some((body.x + column, y));
        }
        render_row(
            f,
            row,
            style,
            body.width,
            Rect {
                y,
                height: 1,
                ..body
            },
            framed,
        );
    }

    caret
}

fn row_caret(row: &Row) -> Option<u16> {
    match row {
        Row::Input {
            value,
            focused: true,
            ..
        } => u16::try_from(Cells::of(value).get()).ok(),
        Row::Field {
            caption,
            value,
            focused: true,
            enabled: true,
        } => Some(field_caret(caption, value)),
        _ => None,
    }
}

/// A red warning dialog: a few sentences and some buttons.
pub(super) fn render_warning(
    f: &mut Frame,
    tracker: &mut crate::mouse::DialogMouse,
    title: &str,
    message: &[String],
    buttons: &[Button],
) {
    let rows: Vec<Row> = message.iter().map(|line| Row::Text(line)).collect();
    render_tracked(f, tracker, title, Ground::Warning, &rows, buttons);
}

/// One body row, drawn according to what it is.
fn render_row(f: &mut Frame, row: &Row, style: Style, width: u16, area: Rect, framed: Rect) {
    let accent = style.fg(palette::HOTKEY).add_modifier(Modifier::BOLD);
    let focused = row.is_focused();
    let enabled = row.is_enabled();
    let line = match row {
        Row::Divider => {
            render_rule(f, style, area.y, framed);
            return;
        }
        Row::Text(text) => Line::from(hotkey_spans(text, style, accent)).centered(),
        Row::Label(text) => Line::from(hotkey_spans(text, style, accent)),
        Row::Input { value, history, .. } => input_line(value, focused, *history, style, width),
        Row::Radio {
            caption,
            options,
            selected,
            ..
        } => radio_line(caption, options, *selected, focused, style),
        Row::Choice { caption, value, .. } | Row::Field { caption, value, .. } => {
            captioned_box(caption, value, focused, enabled, style, accent, width)
        }
        Row::Checkbox { label, checked, .. } => {
            checkbox_line(label, *checked, focused, enabled, style)
        }
    };
    f.render_widget(Paragraph::new(line).style(style), area);
}

impl Row<'_> {
    fn is_focused(&self) -> bool {
        match self {
            Self::Input { focused, .. }
            | Self::Radio { focused, .. }
            | Self::Choice { focused, .. }
            | Self::Field { focused, .. }
            | Self::Checkbox { focused, .. } => *focused,
            Self::Text(_) | Self::Label(_) | Self::Divider => false,
        }
    }

    fn is_enabled(&self) -> bool {
        match self {
            Self::Field { enabled, .. } | Self::Checkbox { enabled, .. } => *enabled,
            _ => true,
        }
    }
}

fn input_line(
    value: &str,
    focused: bool,
    history: bool,
    style: Style,
    width: u16,
) -> Line<'static> {
    // The field spans the box rather than sitting in a fixed-width
    // well: Far's does, and a short well beside empty dialog reads as a
    // layout that failed rather than as a place to type.
    let field = Style::default().bg(palette::FIELD_BG).fg(palette::FIELD_FG);
    let marker = usize::from(history);
    let room = usize::from(width).max(INPUT_WIDTH).saturating_sub(marker);
    let text = format!("{value:<room$}");
    let mut spans = vec![Span::styled(
        text,
        if focused {
            field.add_modifier(Modifier::BOLD)
        } else {
            field
        },
    )];
    if history {
        spans.push(Span::styled(DROP_MARKER.to_string(), style));
    }
    Line::from(spans)
}

fn radio_line(
    caption: &str,
    options: &[&str],
    selected: usize,
    focused: bool,
    style: Style,
) -> Line<'static> {
    let accent = style.fg(palette::HOTKEY).add_modifier(Modifier::BOLD);
    let focus = focused_control_style(style);
    let mut spans = hotkey_spans(caption, style, accent);
    spans.push(Span::styled(" ", style));
    for (index, option) in options.iter().enumerate() {
        let chosen = index == selected;
        // Only the chosen option carries the focus highlight: a row of
        // three reversed options would say the cursor is on all three.
        let base = if focused && chosen { focus } else { style };
        spans.push(Span::styled(
            if chosen { RADIO_ON } else { RADIO_OFF },
            base,
        ));
        spans.extend(hotkey_spans(
            option,
            base,
            accent.bg(base.bg.unwrap_or(palette::DIALOG_BG)),
        ));
    }
    Line::from(spans)
}

fn checkbox_line(
    label: &str,
    checked: bool,
    focused: bool,
    enabled: bool,
    style: Style,
) -> Line<'static> {
    let accent = style.fg(palette::HOTKEY).add_modifier(Modifier::BOLD);
    let focus = focused_control_style(style);
    let base = if !enabled {
        style.fg(palette::DISABLED_FG)
    } else if focused {
        focus
    } else {
        style
    };
    let accent = if enabled {
        accent.bg(base.bg.unwrap_or(palette::DIALOG_BG))
    } else {
        base
    };
    let mark = if checked { 'x' } else { ' ' };
    let mut spans = vec![Span::styled(format!("[{mark}] "), base)];
    spans.extend(hotkey_spans(label, base, accent));
    Line::from(spans)
}

/// A caption and a boxed value on one line, which is how both a drop-down and
/// a captioned text field are drawn.
///
/// One function because the difference between them is what typing does, not
/// what they look like — and two copies of the arithmetic would put the boxes
/// of one group in two different columns.
fn captioned_box<'a>(
    caption: &'a str,
    value: &'a str,
    focused: bool,
    enabled: bool,
    style: Style,
    accent: Style,
    width: u16,
) -> Line<'a> {
    let base = if enabled {
        style
    } else {
        style.fg(palette::DISABLED_FG)
    };
    let gap = caption_gap(caption);
    let mut spans = hotkey_spans(caption, base, if enabled { accent } else { base });
    spans.push(Span::styled(" ".repeat(gap), style));

    let taken = Cells::of(&caption.replace(HOTKEY_MARK, "")).get() + gap;
    let room = usize::from(width)
        .saturating_sub(taken + 1)
        .max(CHOICE_WIDTH);
    let field = Style::default().bg(palette::FIELD_BG).fg(if enabled {
        palette::FIELD_FG
    } else {
        palette::DISABLED_FG
    });
    spans.push(Span::styled(
        format!("{value:<room$}"),
        if focused {
            field.add_modifier(Modifier::BOLD)
        } else {
            field
        },
    ));
    spans.push(Span::styled(DROP_MARKER.to_string(), base));
    Line::from(spans)
}

/// Where the caret goes in a [`Row::Field`], measured the way it is drawn.
pub(super) fn field_caret(caption: &str, value: &str) -> u16 {
    let taken = Cells::of(&caption.replace(HOTKEY_MARK, "")).get() + caption_gap(caption);
    u16::try_from(taken + Cells::of(value).get()).unwrap_or(u16::MAX)
}

/// A rule across the dialog, joined to the frame at both ends.
///
/// Double, like the frame it meets: a single rule ending on a double border
/// leaves two half-joined corners, which is what makes a dialog look drawn by
/// two different programs.
fn render_rule(f: &mut Frame, style: Style, y: u16, framed: Rect) {
    let rule = format!(
        "╠{}╣",
        "═".repeat(usize::from(framed.width.saturating_sub(2)))
    );
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(rule, style))).style(style),
        Rect {
            y,
            height: 1,
            ..framed
        },
    );
}

/// Split a label on its hotkey marker, colouring the letter that follows.
fn hotkey_spans<'a>(label: &str, base: Style, accent: Style) -> Vec<Span<'a>> {
    let Some((before, after)) = label.split_once(HOTKEY_MARK) else {
        return vec![Span::styled(label.to_owned(), base)];
    };
    let mut chars = after.chars();
    let Some(key) = chars.next() else {
        return vec![Span::styled(before.to_owned(), base)];
    };
    vec![
        Span::styled(before.to_owned(), base),
        Span::styled(key.to_string(), accent),
        Span::styled(chars.as_str().to_owned(), base),
    ]
}

/// The buttons, centred, each with its hotkey letter picked out.
fn button_line<'a>(buttons: &[Button], ground: Style) -> Line<'a> {
    let mut spans = Vec::new();
    for button in buttons {
        // The focused button is the one Enter acts on, so it is the one lit up.
        let base = button_style(button, ground);
        let (open, close) = if button.is_default {
            ("{ ", " }")
        } else {
            ("[ ", " ]")
        };
        spans.push(Span::styled(open, base));
        // A disabled button's hotkey is not picked out: the accent means "this
        // letter does something", and here it does not.
        spans.extend(hotkey_spans(
            button.label,
            base,
            if button.is_enabled {
                base.fg(palette::HOTKEY).add_modifier(Modifier::BOLD)
            } else {
                base
            },
        ));
        spans.push(Span::styled(close, base));
        spans.push(Span::styled(BUTTON_GAP, ground));
    }
    spans.pop();
    Line::from(spans).centered()
}

fn button_style(button: &Button, ground: Style) -> Style {
    if !button.is_enabled {
        ground.fg(palette::DISABLED_FG)
    } else if button.is_focused {
        focused_control_style(ground)
    } else {
        ground
    }
}

fn focused_control_style(ground: Style) -> Style {
    let background = if ground.bg == Some(palette::WARNING_BG) {
        palette::WARNING_FOCUS_BG
    } else {
        palette::DIALOG_FOCUS_BG
    };
    ground.bg(background).fg(palette::DIALOG_FG)
}

/// The shadow Far drops behind a dialog, clipped to the screen.
///
/// Half-transparent rather than black: the cells keep their own characters and
/// are darkened in place, so the panel underneath still reads through the
/// shadow the way a real one would. Blanking them instead cuts a black hole in
/// the listing, which says "something is missing here" rather than "something
/// is in front of this".
///
/// Drawn before the dialog and never over it: the two rectangles overlap, and
/// painting the shadow second would darken the dialog's own right edge.
pub(super) fn render_shadow(f: &mut Frame, area: Rect, screen: Rect) {
    /// How much of the original colour survives, out of eight.
    const KEPT: u16 = 3;

    let x = area.x + SHADOW_X;
    let y = area.y + SHADOW_Y;
    if x >= screen.right() || y >= screen.bottom() {
        return;
    }
    let width = area.width.min(screen.right() - x);
    let height = area.height.min(screen.bottom() - y);

    let dim = |colour: Color| match colour {
        Color::Rgb(red, green, blue) => Color::Rgb(
            (u16::from(red) * KEPT / 8) as u8,
            (u16::from(green) * KEPT / 8) as u8,
            (u16::from(blue) * KEPT / 8) as u8,
        ),
        // A named colour has no components to scale, so it goes to the
        // shadow's own darkness rather than being left at full brightness.
        _ => palette::SHADOW,
    };

    let buffer = f.buffer_mut();
    for row in y..y + height {
        for column in x..x + width {
            let cell = &mut buffer[(column, row)];
            let (fg, bg) = (cell.fg, cell.bg);
            cell.set_fg(dim(fg)).set_bg(dim(bg));
        }
    }
}
