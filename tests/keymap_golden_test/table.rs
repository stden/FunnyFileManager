//! Purpose: the golden tables on disk, one TSV per context — written by the
//! generator, replayed line by line against the live resolvers.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::outcome::Context;
use super::probes::Probe;

/// Where the tables live.
pub fn directory() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("keymap_golden")
}

pub fn path(context: &Context) -> PathBuf {
    directory().join(format!("{}.tsv", context.name))
}

/// The table's text: one line per probe, in probe order.
pub fn render(context: &Context, probes: &[Probe]) -> String {
    let mut text = String::new();
    for probe in probes {
        text.push_str(&context.row(probe));
        text.push('\n');
    }
    text
}

pub fn write(context: &Context, probes: &[Probe]) -> io::Result<()> {
    fs::write(path(context), render(context, probes))
}

/// Replays the table against the context: every line, in order, must be the
/// row the resolver gives that key, and no key of the domain may be missing
/// or added. The error names the first line that differs.
pub fn replay(context: &Context, probes: &[Probe]) -> Result<(), String> {
    let file = path(context);
    let file = file.display();
    let text = fs::read_to_string(path(context)).map_err(|error| format!("{file}: {error}"))?;
    let mut lines = text.lines().enumerate();
    for probe in probes {
        let Some((index, line)) = lines.next() else {
            return Err(format!(
                "{file}: ends before `{}`; {} rows expected",
                probe.text,
                probes.len()
            ));
        };
        check(context, probe, line).map_err(|why| format!("{file}:{}: {why}", index + 1))?;
    }
    match lines.next() {
        Some((index, line)) => Err(format!(
            "{file}:{}: `{line}` names no key of the domain",
            index + 1
        )),
        None => Ok(()),
    }
}

/// One line against one probe: the key it names, then the class and the
/// result the resolver gives that key.
fn check(context: &Context, probe: &Probe, line: &str) -> Result<(), String> {
    let mut fields = line.split('\t');
    let (Some(chord), Some(class), Some(result), None) =
        (fields.next(), fields.next(), fields.next(), fields.next())
    else {
        return Err(format!("`{line}` is not chord<TAB>class<TAB>result"));
    };
    if chord != probe.text {
        return Err(format!(
            "names `{chord}` where the domain has `{}`: out of order or edited by hand",
            probe.text
        ));
    }
    let outcome = (context.run)(probe.event);
    let expected_class = context.classify(probe, &outcome).name();
    let expected_result = outcome.to_string();
    if class != expected_class || result != expected_result {
        return Err(format!(
            "`{chord}` is recorded as {class} `{result}`; \
             the resolver gives {expected_class} `{expected_result}`"
        ));
    }
    Ok(())
}
