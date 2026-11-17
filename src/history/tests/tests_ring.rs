//! Purpose: session ring walking — recall, reverse search, `RingIndex` steps, and the session-only persistence state.

use super::*;

// ─── recall and reverse search ────────────────────────────────────────────

#[test]
fn recall_walks_back_from_the_newest_and_stops_at_the_oldest() {
    let mut history = History::default();
    history.record("cd /one");
    history.record("cd /two");
    history.record("cd /three");

    assert_eq!(history.recall(RingIndex::new(0)), Some("cd /three"));
    assert_eq!(history.recall(RingIndex::new(1)), Some("cd /two"));
    assert_eq!(history.recall(RingIndex::new(2)), Some("cd /one"));
    // Past the oldest: `None` is what stops `Up` instead of wrapping around.
    assert_eq!(history.recall(RingIndex::new(3)), None);
}

#[test]
fn recall_of_an_empty_history_yields_nothing() {
    assert_eq!(History::default().recall(RingIndex::new(0)), None);
}

#[test]
fn recall_does_not_show_a_line_twice_from_both_rings() {
    // A durable line executed this run lives in the session ring *and* on
    // disk; walking through it twice would read as a stuck Up key.
    with_xdg(|_| {
        let mut history = History::load();
        history.record("cd /persisted");
        history.record("secret-token-command");

        let reloaded = {
            let mut fresh = History::load();
            fresh.record("cd /persisted");
            fresh
        };
        assert_eq!(reloaded.recall(RingIndex::new(0)), Some("cd /persisted"));
        assert_eq!(
            reloaded.recall(RingIndex::new(1)),
            None,
            "the durable copy of the same line must not repeat"
        );
    });
}

#[test]
fn recall_reaches_persisted_lines_after_the_session_ones() {
    with_xdg(|_| {
        {
            let mut first = History::load();
            first.record("cd /from-disk");
        }
        let mut second = History::load();
        second.record("cd /this-session");

        assert_eq!(second.recall(RingIndex::new(0)), Some("cd /this-session"));
        assert_eq!(second.recall(RingIndex::new(1)), Some("cd /from-disk"));
    });
}

#[test]
fn search_back_finds_a_substring_not_just_a_prefix() {
    let mut history = History::default();
    history.record("grep needle /var/log/syslog");
    history.record("cd /tmp");

    // Unlike `suggest`, which is prefix-only because Right accepts it blind.
    assert_eq!(
        history.search_back("needle", RingIndex::new(0)),
        Some(RingIndex::new(1))
    );
    assert_eq!(history.suggest("needle"), None);
}

#[test]
fn search_back_resumes_past_the_index_it_is_given() {
    let mut history = History::default();
    history.record("cd /alpha");
    history.record("cd /beta");
    history.record("ls");

    assert_eq!(
        history.search_back("cd", RingIndex::new(0)),
        Some(RingIndex::new(1))
    );
    // A second Ctrl+R steps to the next older match.
    assert_eq!(
        history.search_back("cd", RingIndex::new(2)),
        Some(RingIndex::new(2))
    );
    assert_eq!(history.search_back("cd", RingIndex::new(3)), None);
}

#[test]
fn search_back_with_an_empty_query_matches_nothing() {
    let mut history = History::default();
    history.record("cd /tmp");
    // Otherwise a bare Ctrl+R would silently recall the last command.
    assert_eq!(history.search_back("", RingIndex::new(0)), None);
}

#[test]
fn ring_index_steps_in_both_directions_and_stops_at_the_newest() {
    // The walk lives on the type, so Up and Down cannot drift apart in how
    // they move — and `newer()` returning None at the newest entry is what
    // tells the prompt to put the half-typed line back.
    let newest = RingIndex::NEWEST;
    assert_eq!(newest.offset(), 0);
    assert_eq!(newest.newer(), None);
    assert_eq!(newest.older().offset(), 1);
    assert_eq!(newest.older().older().newer(), Some(RingIndex::new(1)));
}

#[test]
fn session_only_is_a_named_state_not_a_missing_path() {
    // `Option<PathBuf>` read as "the path might be missing"; it meant "this
    // history deliberately does not persist", which is what every test and
    // every freshly-constructed App is in.
    assert_eq!(Persistence::default(), Persistence::SessionOnly);
    assert_eq!(Persistence::SessionOnly.file(), None);
    assert_eq!(
        Persistence::File(std::path::PathBuf::from("/tmp/h")).file(),
        Some(std::path::Path::new("/tmp/h"))
    );

    // A default history records for the session and writes nothing.
    let mut history = History::default();
    history.record("cd /tmp");
    assert_eq!(history.recall(RingIndex::NEWEST), Some("cd /tmp"));
}
