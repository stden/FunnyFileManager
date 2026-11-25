//! Purpose: Headless `--things-patch` runner for server patching via Things GraphQL.

pub mod options;

use std::{
    ffi::OsString,
    io::{self, ErrorKind, Write},
    time::{Duration, Instant},
};

use reqwest::blocking::ClientBuilder;

use super::{
    build_plan,
    client::{fetch_with, https_builder},
    is_success_status, is_terminal_status,
    patch::{poll_group_with, start_wave_with, GroupResult},
    Config, Machine, PatchPlan, PatchWave, ThingsError,
};
use options::{resolve_config, CliOptions};

/// Return the remaining CLI arguments when invoked as `ffm --things-patch ...`.
pub fn arguments(args: &[OsString]) -> Option<&[OsString]> {
    let (first, remaining) = args.split_first()?;
    (first == "--things-patch").then_some(remaining)
}

/// Run `--things-patch` using process environment variables and stdout.
pub fn run(args: &[OsString]) -> io::Result<()> {
    let parsed = CliOptions::parse(args)?;
    let config = resolve_config(&parsed)?;
    let mut stdout = io::stdout();
    run_with(https_builder, &config, &parsed, &mut stdout)
}

/// Execute or dry-run a server patch workflow plan against `config`.
pub fn run_with(
    make_builder: fn() -> ClientBuilder,
    config: &Config,
    options: &CliOptions,
    out: &mut impl Write,
) -> io::Result<()> {
    let listing = fetch_with(make_builder, config).map_err(things_error)?;
    let selected: Vec<&Machine> = listing
        .machines
        .iter()
        .filter(|machine| matches_filters(machine, &options.hosts, &options.exclude))
        .collect();
    let plan = build_plan(selected, options.scope, options.reboot, &options.comment);
    write_plan_summary(&plan, out)?;
    if !options.execute {
        writeln!(out, "Dry run: pass --execute to start workflows")?;
        return Ok(());
    }
    if plan.waves.is_empty() {
        return Err(io::Error::new(
            ErrorKind::InvalidInput,
            "No patchable machines matched the selection",
        ));
    }
    execute_waves(make_builder, config, &plan, options, out)
}

fn execute_waves(
    make_builder: fn() -> ClientBuilder,
    config: &Config,
    plan: &PatchPlan,
    options: &CliOptions,
    out: &mut impl Write,
) -> io::Result<()> {
    for wave in &plan.waves {
        let group = start_wave_with(make_builder(), config, wave, plan.reboot, &plan.comment)
            .map_err(things_error)?;
        writeln!(
            out,
            "Started wave {}: group={} status={} executions={}",
            wave.index,
            group.id,
            group.status,
            execution_ids(&group)
        )?;
        if options.wait {
            wait_for_group(make_builder, config, &group.id, options, out)?;
        }
    }
    Ok(())
}

fn wait_for_group(
    make_builder: fn() -> ClientBuilder,
    config: &Config,
    group_id: &str,
    options: &CliOptions,
    out: &mut impl Write,
) -> io::Result<()> {
    wait_for_group_with(
        make_builder,
        config,
        group_id,
        Duration::from_secs(options.poll_secs),
        Duration::from_secs(options.timeout_secs),
        out,
    )
}

pub(super) fn wait_for_group_with(
    make_builder: fn() -> ClientBuilder,
    config: &Config,
    group_id: &str,
    interval: Duration,
    timeout: Duration,
    out: &mut impl Write,
) -> io::Result<()> {
    let start = Instant::now();
    loop {
        let group = poll_group_with(make_builder(), config, group_id).map_err(things_error)?;
        writeln!(out, "Poll group={}: status={}", group.id, group.status)?;
        if is_terminal_status(&group.status) {
            return check_group_outcome(&group);
        }
        if start.elapsed() >= timeout {
            return Err(io::Error::new(
                ErrorKind::TimedOut,
                format!("Timed out waiting for group {}", group.id),
            ));
        }
        if !interval.is_zero() {
            std::thread::sleep(interval);
        }
    }
}

fn check_group_outcome(group: &GroupResult) -> io::Result<()> {
    if is_success_status(&group.status) {
        return Ok(());
    }
    Err(io::Error::other(format!(
        "Group {} finished with non-success status {}",
        group.id, group.status
    )))
}

fn matches_filters(machine: &Machine, hosts: &[String], exclude: &[String]) -> bool {
    let included = hosts.is_empty() || hosts.iter().any(|needle| matches_host(machine, needle));
    let excluded = exclude.iter().any(|needle| matches_host(machine, needle));
    included && !excluded
}

fn matches_host(machine: &Machine, needle: &str) -> bool {
    let target = needle.trim();
    if target.is_empty() {
        return false;
    }
    let short_host = machine.hostname.split('.').next().unwrap_or_default();
    machine.name.eq_ignore_ascii_case(target)
        || machine.hostname.eq_ignore_ascii_case(target)
        || (!short_host.is_empty() && short_host.eq_ignore_ascii_case(target))
}

fn write_plan_summary(plan: &PatchPlan, out: &mut impl Write) -> io::Result<()> {
    for wave in &plan.waves {
        writeln!(out, "{}", format_wave(wave, plan.reboot))?;
    }
    for skipped in &plan.skipped {
        writeln!(out, "Skipped: {skipped}")?;
    }
    Ok(())
}

fn format_wave(wave: &PatchWave, reboot: bool) -> String {
    let names: Vec<String> = wave
        .targets
        .iter()
        .map(|target| format!("{} [{}]", target.name, target.id))
        .collect();
    format!(
        "Wave {} ({}, reboot={}): {}",
        wave.index,
        wave.kind.label(),
        reboot,
        names.join(", ")
    )
}

fn execution_ids(group: &GroupResult) -> String {
    let ids: Vec<&str> = group
        .executions
        .iter()
        .map(|exec| exec.id.as_str())
        .collect();
    ids.join(",")
}

fn things_error(error: ThingsError) -> io::Error {
    io::Error::other(error.message())
}
