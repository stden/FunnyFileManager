//! Purpose: the `viewer` table — the F3 viewer and the report screens. The
//! quick-view panel scrolls with the same scroll commands.

use crate::action::chord::NamedKey::{Down, End, Enter, Esc, Home, Left, PgDn, PgUp, Right, Up};
use crate::action::viewer::{ViewerCommand, ViewerScroll};
use crate::keymap::chords::{ctrl, f, named};
use crate::keymap::Origin::FarSame;
use crate::keymap::{row, Context, Group, Table};

use ViewerCommand::Scroll;

pub const VIEWER: Table<ViewerCommand> = Table {
    context: Context::Viewer,
    groups: &[CLOSING, SCROLLING],
};

const CLOSING: Group<ViewerCommand> = Group {
    title: "Closing",
    rows: &[row(
        "close",
        ViewerCommand::Close,
        &[named(Esc), named(Enter), f(3), f(10)],
    )
    .note(FarSame, "Close the viewer")
    .exit()],
};

const SCROLLING: Group<ViewerCommand> = Group {
    title: "Scrolling",
    rows: &[
        row("line_up", Scroll(ViewerScroll::LineUp), &[named(Up)]).note(FarSame, "One line up"),
        row("line_down", Scroll(ViewerScroll::LineDown), &[named(Down)])
            .note(FarSame, "One line down"),
        row("page_up", Scroll(ViewerScroll::PageUp), &[named(PgUp)]).note(FarSame, "One screen up"),
        row("page_down", Scroll(ViewerScroll::PageDown), &[named(PgDn)])
            .note(FarSame, "One screen down"),
        row("top", Scroll(ViewerScroll::Top), &[named(Home)]).note(FarSame, "The first screen"),
        row("bottom", Scroll(ViewerScroll::Bottom), &[named(End)]).note(FarSame, "The last screen"),
        row("scroll_left", Scroll(ViewerScroll::Left), &[named(Left)])
            .note(FarSame, "One column left"),
        row("scroll_right", Scroll(ViewerScroll::Right), &[named(Right)])
            .note(FarSame, "One column right"),
        row(
            "scroll_left_wide",
            Scroll(ViewerScroll::LeftWide),
            &[ctrl(named(Left))],
        )
        .note(FarSame, "Twenty columns left"),
        row(
            "scroll_right_wide",
            Scroll(ViewerScroll::RightWide),
            &[ctrl(named(Right))],
        )
        .note(FarSame, "Twenty columns right"),
    ],
};
