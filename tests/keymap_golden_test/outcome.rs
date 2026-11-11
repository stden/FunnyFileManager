//! Purpose: what a context does with one key — a consumed command, nothing,
//! a fall-through or a refusal — and the class the golden table gives the
//! row, assigned mechanically as the design spec (§9.1) orders them.

use std::fmt;

use crossterm::event::KeyEvent;

use super::probes::Probe;

/// What a context, and the popup layered over it, does with a key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The context's own command, in its debug form.
    Consumed(String),
    /// The key is swallowed: the context has nothing for it and lets
    /// nothing past.
    Nothing,
    /// The key goes on to the parent context, which answers with this.
    FallsThrough(String),
    /// A `NetBox` panel refuses the panels action that would act on the
    /// entries hidden beneath it.
    Refuses(String),
}

impl fmt::Display for Outcome {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Consumed(command) => out.write_str(command),
            Self::Nothing => out.write_str("nothing"),
            Self::FallsThrough(action) => write!(out, "-> {action}"),
            Self::Refuses(action) => write!(out, "refuses {action}"),
        }
    }
}

/// How a row came by its result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Class {
    /// The context's fallback: a family command (`Insert`, `Search`, the
    /// panels' command line and quick search), `Ignore`, nothing, a
    /// fall-through or a refusal.
    Fallback,
    /// A named command that a strict subset of the chord's modifiers also
    /// yields: `Shift+F5` copies because `F5` does (§2.3's leniency).
    Lenient,
    /// Any other named command.
    Bound,
}

impl Class {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Fallback => "fallback",
            Self::Lenient => "lenient",
            Self::Bound => "bound",
        }
    }
}

/// One screen's keys: its name in `ffm.yml`, what its resolver (with the
/// popup layered over it) does with an event, and the command §6.3's
/// fallback gives an event when that fallback is a command of the
/// context's own — `Nothing` where the fallback consumes no command.
pub struct Context {
    pub name: &'static str,
    pub run: fn(KeyEvent) -> Outcome,
    pub family: fn(KeyEvent) -> Outcome,
}

impl Context {
    /// The row's class. Only a consumed command can be `bound` or
    /// `lenient`; nothing, a fall-through and a refusal are the fallback.
    pub fn classify(&self, probe: &Probe, outcome: &Outcome) -> Class {
        if !matches!(outcome, Outcome::Consumed(_)) || *outcome == (self.family)(probe.event) {
            return Class::Fallback;
        }
        let lenient = probe
            .subsets
            .iter()
            .any(|&subset| (self.run)(subset) == *outcome);
        if lenient {
            Class::Lenient
        } else {
            Class::Bound
        }
    }

    /// The table's line for the key: `chord<TAB>class<TAB>result`.
    pub fn row(&self, probe: &Probe) -> String {
        let outcome = (self.run)(probe.event);
        let class = self.classify(probe, &outcome);
        format!("{}\t{}\t{outcome}", probe.text, class.name())
    }
}

/// A command the context consumed.
pub fn consumed(command: &impl fmt::Debug) -> Outcome {
    Outcome::Consumed(debug(command))
}

/// The debug form of a command, which the tables record until the keymap
/// gives each command a name (design spec §9.1).
pub fn debug(value: &impl fmt::Debug) -> String {
    format!("{value:?}")
}
