//! Purpose: Background worker launcher helpers for Things read and patch jobs.

use std::sync::mpsc;

use super::{Answer, JobAnswer, PatchAnswer};
use crate::things::{Config, PatchPlan};

pub(super) fn spawn_read(
    sender: mpsc::Sender<JobAnswer>,
    config: Config,
    work: impl FnOnce(Config) -> Answer + Send + 'static,
) -> std::io::Result<std::thread::JoinHandle<()>> {
    std::thread::Builder::new()
        .name("ffm-things".to_owned())
        .spawn(move || drop(sender.send(JobAnswer::Read(work(config)))))
}

pub(super) fn spawn_patch(
    sender: mpsc::Sender<JobAnswer>,
    config: Config,
    plan: PatchPlan,
    work: impl FnOnce(Config, PatchPlan) -> PatchAnswer + Send + 'static,
) -> std::io::Result<std::thread::JoinHandle<()>> {
    std::thread::Builder::new()
        .name("ffm-things-patch".to_owned())
        .spawn(move || drop(sender.send(JobAnswer::Patch(work(config, plan)))))
}
