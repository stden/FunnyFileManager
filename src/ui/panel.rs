//! Purpose: Draws one file panel — its columns, header, rows and footer.
//!
//! Split out of `ui.rs` when the Far-style columns pushed that file past the
//! 1000-line cap `tests/file_line_limit_test.rs` enforces. The boundary is not
//! arbitrary: everything here is about the inside of a panel, and `ui.rs` keeps
//! the screen it sits on.

use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use super::palette;
use super::{far_block, head_cells, tail_cells, take_cells, Cells, FAR_BLUE, PANEL_BORDERS};
use super::{PANEL_COLUMN_HEADER, PANEL_COLUMN_RULE, PANEL_INFO_ROW, PANEL_NON_ENTRY_ROWS};
use crate::models::{App, DirEntry, Panel, PanelView, ShortNames, SortMode};

mod modes;

/// Right-aligned width of the Size column: "Folder", "Up", or a byte count.
///
/// Exactly six cells, because that is the width of the word `Folder` — the
/// column is sized to its widest label, so "Folder" fills it edge to edge with
/// no stray leading blank, and every number right-aligns against the same edge.
const SIZE_COLUMN_WIDTH: Cells = Cells(6);

/// Width of the Date column, which is exactly `MM/DD/YY`.
const DATE_COLUMN_WIDTH: Cells = Cells(8);

/// Width of the Time column, which is exactly `HH:MM`.
const TIME_COLUMN_WIDTH: Cells = Cells(5);

/// The rule Far draws between panel columns, and the one cell it costs.
const COLUMN_SEPARATOR: &str = "\u{2502}";
const COLUMN_SEPARATOR_WIDTH: Cells = Cells(1);

/// The row that leads out of a directory, which the Size column labels "Up".
const PARENT_DIR: &str = "..";

/// Narrowest name column the panel will render.
///
/// Without a floor, the fixed-width size column has absolute priority and a
/// narrow terminal (30 columns → 15 per panel) leaves the name exactly zero
/// cells, so `fit_name` returns "" for every entry and the panel becomes a
/// column of icons and sizes with no filenames at all. A file manager that
/// cannot show file names is useless, so the size column yields instead.
const MIN_NAME_WIDTH: Cells = Cells(8);

/// The columns Far lays out to the right of the name, in its own order.
///
/// A slice rather than three booleans because they are dropped as a group from
/// the right on a narrow panel (see [`panel_layout`]), and an ordered list is
/// what "drop the last one" is a statement about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum PanelColumn {
    Size,
    Date,
    Time,
}

impl PanelColumn {
    const fn width(self) -> Cells {
        match self {
            Self::Size => SIZE_COLUMN_WIDTH,
            Self::Date => DATE_COLUMN_WIDTH,
            Self::Time => TIME_COLUMN_WIDTH,
        }
    }

    const fn title(self) -> &'static str {
        match self {
            Self::Size => "Size",
            Self::Date => "Date",
            Self::Time => "Time",
        }
    }

    /// This column's text for one row.
    fn cell(self, entry: &DirEntry) -> String {
        match self {
            Self::Size => size_field(entry),
            Self::Date => entry.modified.date(),
            Self::Time => entry.modified.time(),
        }
    }

    /// Place `cell` in this column's cells.
    ///
    /// Every column right-aligns and every column is sized to its content, so
    /// nothing carries a stray blank: `Folder` and `MM/DD/YY` and `HH:MM` each
    /// fill their column exactly, and a byte count lines its digits up against
    /// the same right edge.
    fn lay_out(self, cell: &str) -> String {
        right_cells(cell, self.width())
    }
}

/// `static`, not `const`: the layout hands out `&'static [PanelColumn]`
/// sub-slices of it, and indexing a `const` would borrow a temporary.
static PANEL_COLUMNS: [PanelColumn; 3] = [PanelColumn::Size, PanelColumn::Date, PanelColumn::Time];

/// The columns each of Far's table views puts behind every name, or `None`
/// for a view [`modes`] draws instead.
///
/// Brief and Medium have none — their names take the width — and the info
/// row shows the current entry's instead (see [`info_columns`]).
fn table_columns(view: PanelView) -> Option<&'static [PanelColumn]> {
    match view {
        PanelView::Brief | PanelView::Medium => Some(&[]),
        PanelView::Full => Some(&PANEL_COLUMNS),
        PanelView::Wide => Some(&PANEL_COLUMNS[..1]),
        PanelView::Detailed
        | PanelView::Descriptions
        | PanelView::LongDescriptions
        | PanelView::Owners
        | PanelView::Links
        | PanelView::Alternative => None,
    }
}

/// What the info row adds after the current name: the columns a view's table
/// leaves out. All three under Brief and Medium, Date and Time under Wide,
/// nothing under Full — the rule Far's own status line follows.
fn info_columns(shown: &[PanelColumn]) -> Vec<PanelColumn> {
    PANEL_COLUMNS
        .iter()
        .copied()
        .filter(|column| !shown.contains(column))
        .collect()
}

/// How one panel's interior width is shared between names and columns.
///
/// The panel is `stripes` tables side by side — Brief's three, Medium's two,
/// one for Full and Wide — each a name followed by `columns`. The entries run
/// down the first stripe and on into the next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PanelLayout {
    pub(super) stripes: usize,
    /// The name width in every stripe but the last.
    pub(super) name: Cells,
    /// The last stripe's name width. It also takes the cells an uneven split
    /// leaves over, so every row reaches the frame.
    pub(super) last_name: Cells,
    pub(super) columns: &'static [PanelColumn],
}

impl PanelLayout {
    const fn name_width(self, stripe: usize) -> Cells {
        if stripe + 1 == self.stripes {
            self.last_name
        } else {
            self.name
        }
    }

    /// The cells `stripe` spans: its name and every column behind its rule.
    fn stripe_width(self, stripe: usize) -> Cells {
        self.name_width(stripe) + columns_width(self.columns)
    }
}

/// The cells `columns` take behind a name, each with its rule.
fn columns_width(columns: &[PanelColumn]) -> Cells {
    columns
        .iter()
        .map(|column| column.width() + COLUMN_SEPARATOR_WIDTH)
        .sum()
}

/// The layout of a `view` panel `interior` cells wide.
///
/// The stripes share what the rules between them leave. Within a stripe, the
/// widest column set whose leftovers still clear [`MIN_NAME_WIDTH`] is kept:
/// Far shows Name | Size | Date | Time, a 40-column panel cannot, and the name
/// is the column a file manager exists to show, so the trailing columns are
/// dropped from the right — Time first, then Date, then Size — until the name
/// fits. Dropping from the left instead would leave a Time column beside
/// nameless rows.
pub(super) fn panel_layout(view: PanelView, interior: Cells) -> PanelLayout {
    let stripes = view.columns();
    let rules = Cells::new(COLUMN_SEPARATOR_WIDTH.get() * (stripes - 1));
    let names = interior.saturating_sub(rules).get();
    let name = Cells::new(names / stripes);
    let last_name = Cells::new(names - name.get() * (stripes - 1));

    let declared = table_columns(view).unwrap_or_default();
    let taken = (0..=declared.len())
        .rev()
        .find(|taken| name.saturating_sub(columns_width(&declared[..*taken])) >= MIN_NAME_WIDTH)
        .unwrap_or(0);
    let columns = &declared[..taken];
    let spent = columns_width(columns);
    PanelLayout {
        stripes,
        name: name.saturating_sub(spent),
        last_name: last_name.saturating_sub(spent),
        columns,
    }
}

/// Which stripe of a `view` panel `interior` cells wide covers interior cell
/// `x` — the mouse's reading of [`panel_layout`], so a click lands on what was
/// drawn. The rule between two stripes belongs to the one on its left.
pub fn stripe_at(view: PanelView, interior: Cells, x: usize) -> usize {
    let layout = panel_layout(view, interior);
    let mut end = 0;
    for stripe in 0..layout.stripes {
        end += layout.stripe_width(stripe).get() + COLUMN_SEPARATOR_WIDTH.get();
        if x < end {
            return stripe;
        }
    }
    layout.stripes - 1
}

/// Far splits a file's name at its final dot and right-aligns the extension
/// against the Size column, so a listing lines up by type without sorting.
///
/// Directories are never split — `.cargo` is a name, not an extension — and
/// neither is a leading dot, which starts a hidden file rather than ending an
/// empty one.
pub(super) fn split_extension(entry: &DirEntry) -> (&str, &str) {
    if entry.kind.is_dir() {
        return (&entry.name, "");
    }
    match entry.name.rfind('.') {
        Some(0) | None => (&entry.name, ""),
        Some(dot) => (&entry.name[..dot], &entry.name[dot + 1..]),
    }
}

/// First `width` cells of a panel name, marked with Far's `}` when cut.
///
/// A distinct marker from [`head_cells`]' ellipsis on purpose: `}` cannot
/// occur mid-name often enough to be mistaken for one, and it is the glyph a
/// Far user already reads as "there is more here".
pub(super) fn clip_name(name: &str, width: Cells) -> String {
    if Cells::of(name) <= width {
        return name.to_string();
    }
    if width.is_zero() {
        return String::new();
    }
    let (mut cut, _) = take_cells(name, width.saturating_sub(Cells(1)));
    cut.push('}');
    cut
}

/// `text` padded with blanks to exactly `width` cells.
pub(super) fn pad_cells(text: &str, width: Cells) -> String {
    let padding = " ".repeat(width.saturating_sub(Cells::of(text)).get());
    format!("{text}{padding}")
}

/// `text` right-aligned in exactly `width` cells, as every column but the name
/// is. `pub(super)` because the drive dialog aligns its size columns the same
/// way rather than keeping a copy.
pub(super) fn right_cells(text: &str, width: Cells) -> String {
    let fitted = head_cells(text, width);
    let padding = " ".repeat(width.saturating_sub(Cells::of(&fitted)).get());
    format!("{padding}{fitted}")
}

/// `text` centred in exactly `width` cells — the column headers' alignment.
fn center_cells(text: &str, width: Cells) -> String {
    let fitted = head_cells(text, width);
    let slack = width.saturating_sub(Cells::of(&fitted)).get();
    let left = slack / 2;
    format!("{}{fitted}{}", " ".repeat(left), " ".repeat(slack - left))
}

/// The name column for one row: base name left, extension right.
pub(super) fn name_field(entry: &DirEntry, width: Cells) -> String {
    let (base, extension) = split_extension(entry);
    let extension_width = Cells::of(extension);
    // Splitting costs the extension plus at least one blank column. Where the
    // panel cannot pay that, the undivided name keeps the room instead: a bare
    // right-aligned "rs" identifies a file far less than a truncated name does.
    if extension.is_empty() || extension_width + Cells(2) > width {
        return pad_cells(&clip_name(&entry.name, width), width);
    }
    let base_room = width.saturating_sub(extension_width + Cells(1));
    let base = clip_name(base, base_room);
    let gap = width
        .saturating_sub(Cells::of(&base) + extension_width)
        .get();
    format!("{base}{}{extension}", " ".repeat(gap))
}

/// The Size column's text: Far names what a row *is* before it measures it.
pub(super) fn size_field(entry: &DirEntry) -> String {
    if entry.name == PARENT_DIR {
        return "Up".to_owned();
    }
    if entry.kind.is_dir() {
        return "Folder".to_owned();
    }
    let bytes = entry.size.bytes();
    let exact = bytes.to_string();
    // Far prints the byte count itself and only abbreviates what will not fit,
    // so the common case stays a number the user can compare digit by digit.
    if Cells::of(&exact) <= SIZE_COLUMN_WIDTH {
        exact
    } else {
        abbreviate_bytes(bytes)
    }
}

/// Far's compact byte count — "381 K", "49.4 G": the largest unit that keeps
/// the value under 1000, carrying a decimal only while there is room for one.
pub(super) fn abbreviate_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["", "K", "M", "G", "T"];

    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1000.0 && unit + 1 < UNITS.len() {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        return bytes.to_string();
    }
    if value >= 100.0 {
        format!("{value:.0} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

/// The colour Far gives a row, in the order its rules override each other.
///
/// Marked first — a mark is the user's own statement about the row and must
/// survive whatever the row is. Hidden next, because a dimmed row is dimmed
/// whether it is a folder, a program or a file. Only then does the entry's own
/// nature get a say.
fn entry_colour(entry: &DirEntry, is_marked: bool, hidden: bool) -> Color {
    if is_marked {
        return palette::MARKED;
    }
    if hidden {
        return palette::HIDDEN;
    }
    if entry.kind.is_dir() {
        return palette::DIRECTORY;
    }
    if entry.executable {
        return palette::EXECUTABLE;
    }
    palette::FILE
}

/// The style for a row's name: its colour, bold where Far bolds it.
fn entry_style(entry: &DirEntry, is_marked: bool, hidden: bool) -> Style {
    let style = Style::default().fg(entry_colour(entry, is_marked, hidden));
    if hidden {
        return style;
    }
    if is_marked || entry.kind.is_dir() {
        return style.add_modifier(Modifier::BOLD);
    }
    style
}

/// The column-title row Far draws directly under the top border.
///
/// The leading letter is the sort mode — `n`ame, `e`xtension, `s`ize and the
/// rest of [`SortMode::letter`] — in the corner Far puts it, which is the only
/// place the panel says how it is ordered.
fn header_line(sort_mode: SortMode, layout: PanelLayout) -> Line<'static> {
    // Bright white, like the values under it — Far reserves cyan for the frame
    // and the rules, not for text.
    let style = Style::default().fg(palette::DATA).bg(FAR_BLUE);

    let mut spans = Vec::new();
    for stripe in 0..layout.stripes {
        let name = layout.name_width(stripe);
        let title = if stripe == 0 {
            // Only the first title gives up a cell to the letter.
            let title = center_cells("Name", name.saturating_sub(Cells(1)));
            format!("{}{title}", sort_mode.letter())
        } else {
            spans.push(Span::styled(COLUMN_SEPARATOR, style));
            center_cells("Name", name)
        };
        spans.push(Span::styled(title, style));
        for column in layout.columns {
            spans.push(Span::styled(COLUMN_SEPARATOR, style));
            spans.push(Span::styled(
                center_cells(column.title(), column.width()),
                style,
            ));
        }
    }
    Line::from(spans)
}

/// The colour of the column rules, which is the frame's.
fn separator_style() -> Style {
    Style::default().fg(palette::FRAME)
}

/// The style of the entry under the active panel's cursor.
fn cursor_style() -> Style {
    Style::default()
        .bg(palette::CURSOR_BG)
        .fg(palette::CURSOR_FG)
        .add_modifier(Modifier::BOLD)
}

/// A stripe with no entry left to show: nothing in it but the column rules.
///
/// What makes the panel read as a table below the last file: Far's rules run
/// to the divider, and a rule that stops where the data stops leaves the frame
/// looking broken in every directory with fewer files than rows.
fn empty_cell(name: Cells, columns: &[PanelColumn]) -> Span<'static> {
    let mut cell = " ".repeat(name.get());
    for column in columns {
        cell.push_str(COLUMN_SEPARATOR);
        cell.push_str(&" ".repeat(column.width().get()));
    }
    Span::styled(cell, separator_style())
}

/// One entry in one stripe: the name, then each column behind its rule.
fn entry_cell(
    panel: &Panel,
    entry: &DirEntry,
    name: Cells,
    columns: &[PanelColumn],
) -> Vec<Span<'static>> {
    let is_marked = panel.marked.contains(entry.name.as_str());
    let hidden = panel.is_hidden(&entry.name);
    let displayed = displayed_entry(modes::short_names(panel).as_ref(), entry);
    let name_style = entry_style(entry, is_marked, hidden);
    // The data columns are plain white — reading down them for meaning would
    // find none — except where the row as a whole is making a statement: a
    // marked row is yellow throughout and a hidden one is dimmed throughout,
    // and a bright date beside a recessed name would break both.
    let data_style = Style::default().fg(if is_marked || hidden {
        entry_colour(entry, is_marked, hidden)
    } else {
        palette::DATA
    });

    let mut spans = vec![Span::styled(name_field(&displayed, name), name_style)];
    for column in columns {
        spans.push(Span::styled(COLUMN_SEPARATOR, separator_style()));
        spans.push(Span::styled(
            column.lay_out(&column.cell(&displayed)),
            data_style,
        ));
    }
    spans
}

/// The rule that closes the column separators above the info row.
///
/// Far does not let the column lines run into the frame: they stop on a
/// horizontal divider whose `┴` junctions say which column each one belonged
/// to. It spans the whole panel width, border columns included, so the two ends
/// meet the double frame as `╟` and `╢` rather than leaving a gap.
fn column_rule(layout: PanelLayout) -> String {
    let mut rule = String::from("╟");
    for stripe in 0..layout.stripes {
        if stripe > 0 {
            rule.push('┴');
        }
        rule.push_str(&"─".repeat(layout.name_width(stripe).get()));
        for column in layout.columns {
            rule.push('┴');
            rule.push_str(&"─".repeat(column.width().get()));
        }
    }
    rule.push('╢');
    rule
}

/// The totals Far prints into the panel's bottom border.
///
/// Counts files rather than rows: the directories and the ".." link are not
/// what "381 K" is the size of, so including them in the denominator would
/// make the two halves of `(10/30)` measure different things.
pub(super) fn panel_totals(panel: &Panel) -> String {
    let files: Vec<&DirEntry> = panel
        .entries
        .iter()
        .filter(|entry| !entry.kind.is_dir())
        .collect();
    let bytes: u64 = files.iter().map(|entry| entry.size.bytes()).sum();
    format!(
        " {} ({}/{}) ",
        abbreviate_bytes(bytes),
        files.len(),
        panel.entries.len()
    )
}

fn panel_block(
    panel: &Panel,
    interior: Cells,
    is_active: bool,
    border_style: Style,
) -> ratatui::widgets::Block<'static> {
    let mut block = far_block()
        .border_style(border_style)
        .style(Style::default().bg(FAR_BLUE))
        .title(panel_title(panel, interior, is_active))
        .title_bottom(
            Line::from(Span::styled(
                panel_totals(panel),
                Style::default().fg(Color::White).bg(FAR_BLUE),
            ))
            .centered(),
        );
    if let Some(free) = panel.available_space() {
        block = block.title_bottom(
            Line::from(Span::styled(
                format!(" {} ", abbreviate_bytes(free.bytes())),
                Style::default().fg(Color::White).bg(FAR_BLUE),
            ))
            .right_aligned(),
        );
    }

    block
}

fn panel_title(panel: &Panel, interior: Cells, is_active: bool) -> Line<'static> {
    // Far highlights the active panel's path; an unhighlighted title is how
    // the inactive panel says so even when neither has a visible cursor bar.
    let title_style = if is_active {
        Style::default()
            .fg(palette::HIGHLIGHT_FG)
            .bg(palette::HIGHLIGHT_BG)
    } else {
        Style::default().fg(palette::FRAME).bg(FAR_BLUE)
    };

    // Far's `*` before the path: the listing is not the whole directory.
    let filtered = if panel.filter.is_active() { "*" } else { "" };
    Line::from(Span::styled(
        format!(
            " {filtered}{} ",
            tail_cells(&panel.path.display().to_string(), interior)
        ),
        title_style,
    ))
    .centered()
}

pub(super) fn render_panel(
    f: &mut Frame,
    panel: &mut Panel,
    area: Rect,
    is_active: bool,
    footer: Option<&str>,
) {
    let border_style = if is_active {
        Style::default()
            .fg(palette::FRAME)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(palette::FRAME)
    };
    let interior = Cells::columns(area.width.saturating_sub(PANEL_BORDERS));
    let block = panel_block(panel, interior, is_active, border_style);

    let inner = block.inner(area);
    f.render_widget(block, area);
    if inner.height == 0 {
        return;
    }

    render_panel_contents(f, panel, area, inner, is_active, footer, border_style);
}

fn render_panel_contents(
    f: &mut Frame,
    panel: &mut Panel,
    area: Rect,
    inner: Rect,
    is_active: bool,
    footer: Option<&str>,
    border_style: Style,
) {
    let Some(shown) = table_columns(panel.view) else {
        modes::render(f, panel, inner, is_active);
        render_panel_info(f, panel, inner, footer, &[]);
        return;
    };
    let layout = panel_layout(panel.view, Cells::columns(inner.width));
    f.render_widget(
        Paragraph::new(header_line(panel.sort_mode, layout)).style(Style::default().bg(FAR_BLUE)),
        Rect { height: 1, ..inner },
    );
    if inner.height < PANEL_NON_ENTRY_ROWS {
        return;
    }

    let rows = inner.height - PANEL_NON_ENTRY_ROWS;

    let body = Rect {
        y: inner.y + PANEL_COLUMN_HEADER,
        height: rows,
        ..inner
    };
    render_entries(f, panel, body, layout, is_active);

    // The rule spans `area`, not `inner`: its two ends are the frame cells the
    // block already drew, and overwriting them is what turns a floating dash
    // into a divider joined to the border.
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(column_rule(layout), border_style)))
            .style(Style::default().bg(FAR_BLUE)),
        Rect {
            y: inner.y + inner.height - PANEL_COLUMN_RULE - PANEL_INFO_ROW,
            height: PANEL_COLUMN_RULE,
            ..area
        },
    );

    render_panel_info(f, panel, inner, footer, &info_columns(shown));
}

fn render_entries(
    f: &mut Frame,
    panel: &mut Panel,
    body: Rect,
    layout: PanelLayout,
    is_active: bool,
) {
    // Only the rows that will be drawn are built.
    //
    // Formatting every entry costs a name, a size, a date and a time per
    // entry, of which forty are used. It is linear in the directory rather
    // than in the screen, and at a hundred thousand files that was a tenth of
    // a second per keystroke — the panel redraws on every one.
    //
    // The window counts entries, not rows: Far scrolls a multi-column view one
    // entry at a time, reflowing the listing through its stripes.
    let rows = usize::from(body.height);
    let window = visible_window(
        panel.entries.len(),
        rows * layout.stripes,
        panel.list_state_mut().offset(),
        panel.selected(),
    );
    *panel.list_state_mut().offset_mut() = window.start;

    // Far draws no cursor on the inactive panel: two highlighted rows would
    // give the screen two answers to "what does Enter act on".
    let cursor = panel.selected().filter(|_| is_active);
    let panel = &*panel;
    let lines: Vec<Line> = (0..rows)
        .map(|row| {
            let cells = (0..layout.stripes).map(|stripe| {
                Some(window.start + stripe * rows + row).filter(|index| window.contains(index))
            });
            body_line(panel, layout, cells, cursor)
        })
        .collect();
    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(FAR_BLUE)),
        body,
    );
}

/// One body row: the entry each stripe shows on it, or — where the listing
/// has run out — just the column rules, which run the full height of the body.
fn body_line(
    panel: &Panel,
    layout: PanelLayout,
    cells: impl Iterator<Item = Option<usize>>,
    cursor: Option<usize>,
) -> Line<'static> {
    let mut spans = Vec::new();
    for (stripe, index) in cells.enumerate() {
        if stripe > 0 {
            spans.push(Span::styled(COLUMN_SEPARATOR, separator_style()));
        }
        let name = layout.name_width(stripe);
        let Some(index) = index else {
            spans.push(empty_cell(name, layout.columns));
            continue;
        };
        let cell = entry_cell(panel, &panel.entries[index], name, layout.columns);
        // The cursor covers its own stripe only, never the rule beside it.
        if cursor == Some(index) {
            spans.extend(
                cell.into_iter()
                    .map(|span| span.patch_style(cursor_style())),
            );
        } else {
            spans.extend(cell);
        }
    }
    Line::from(spans)
}

/// The entry as the row shows it: itself, or a copy under its 8.3 name while
/// the panel shows short names, with anything the terminal will not draw
/// replaced. Only the name changes — marks, hiding and every operation still
/// go by the real one.
fn displayed_entry<'a>(
    short_names: Option<&ShortNames>,
    entry: &'a DirEntry,
) -> std::borrow::Cow<'a, DirEntry> {
    let short = short_names.and_then(|table| table.get(&entry.name));
    let shown = short.map_or(entry.name.as_str(), crate::models::ShortName::as_str);
    let printable = crate::ui::printable_name(shown);
    if short.is_none() && matches!(printable, std::borrow::Cow::Borrowed(_)) {
        return std::borrow::Cow::Borrowed(entry);
    }
    let mut displayed = entry.clone();
    displayed.name = printable.into_owned();
    std::borrow::Cow::Owned(displayed)
}

/// The row above the bottom border, which Far gives to the current entry's
/// full name — the one place a name too long for its column can still be read.
///
/// It doubles as this UI's message line. Far has no status bar to lose, so
/// removing ours left `status_text` (the only channel a failed copy or a
/// refused view has) with nowhere to go; it takes precedence here, and quick
/// search takes precedence over it, because both are about what the user is
/// doing right now while the name merely repeats the highlighted row.
///
/// A view whose table leaves out some of Size, Date and Time adds the current
/// entry's after the name (see [`info_columns`]), as Far's status line does.
fn render_panel_info(
    f: &mut Frame,
    panel: &Panel,
    inner: Rect,
    footer: Option<&str>,
    details: &[PanelColumn],
) {
    let width = Cells::columns(inner.width);
    let text = match (footer, panel.get_selected_entry()) {
        (Some(message), _) => head_cells(message, width),
        (None, Some(entry)) => info_text(entry, details, width),
        (None, None) => String::new(),
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            text,
            Style::default().fg(Color::LightCyan).bg(FAR_BLUE),
        ))),
        Rect {
            y: inner.y + inner.height - PANEL_INFO_ROW,
            height: PANEL_INFO_ROW,
            ..inner
        },
    );
}

/// The current entry's name, then its `details` right-aligned — what a view
/// with no room for those columns shows of them.
///
/// The name is what the row is for, so where both do not fit the details
/// give way rather than the name falling under [`MIN_NAME_WIDTH`].
fn info_text(entry: &DirEntry, details: &[PanelColumn], width: Cells) -> String {
    let shown = crate::ui::printable_name(&entry.name);
    let mut columns = String::new();
    for column in details {
        columns.push(' ');
        columns.push_str(&column.lay_out(&column.cell(entry)));
    }
    let name = width.saturating_sub(Cells::of(&columns));
    if columns.is_empty() || name < MIN_NAME_WIDTH {
        return head_cells(&shown, width);
    }
    format!("{}{columns}", pad_cells(&head_cells(&shown, name), name))
}

/// What the active panel's info row says, when it is not just the entry name.
pub(super) fn panel_footer(app: &App) -> Option<String> {
    if let Some(search) = app.quick_search.query() {
        return Some(format!("Search: {search}_"));
    }
    // Status lines quote file names — "Viewing x", "Saved x", a failed copy's
    // path — so the message gets the same treatment the name it replaces does.
    app.status_news()
        .map(|message| crate::ui::printable_name(&message).into_owned())
}

/// Which entries a panel body `rows` tall will actually show.
///
/// The same rule `List` applies to its own state, worked out before the items
/// are built rather than after: the offset holds unless the cursor has moved
/// outside it, and then it moves by just enough to bring the cursor back.
///
/// Returned as a range so the caller can slice with it, and pure so that the
/// scrolling behaviour can be checked without a terminal.
pub(super) fn visible_window(
    len: usize,
    rows: usize,
    offset: usize,
    selected: Option<usize>,
) -> std::ops::Range<usize> {
    if len == 0 || rows == 0 {
        return 0..0;
    }
    let mut start = offset.min(len.saturating_sub(1));
    if let Some(selected) = selected.filter(|at| *at < len) {
        if selected < start {
            // Scrolled up past the cursor: the cursor becomes the first row.
            start = selected;
        } else if selected >= start + rows {
            // …and down past it: the cursor becomes the last.
            start = selected + 1 - rows;
        }
    }
    // A short listing is shown from the top rather than with blank rows above
    // it, which is what a stale offset into a directory that just shrank would
    // otherwise leave behind.
    start = start.min(len.saturating_sub(rows));
    start..(start + rows).min(len)
}

#[cfg(test)]
mod tests_header;
#[cfg(test)]
mod tests_snapshot;
#[cfg(test)]
mod tests_views;
#[cfg(test)]
mod tests_window;
