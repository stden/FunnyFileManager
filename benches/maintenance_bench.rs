//! Measure opening maintenance confirmations through the Commands menu.

use criterion::{criterion_group, criterion_main, Criterion};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use funny_file_manager::{
    action::Action,
    dispatch_key,
    menu::{Heading, Menu, MenuCommand},
    models::{App, ChoiceTarget, MaintenanceCommand, MenuDialog, Overlay, Panel, VolumeOperation},
    WritableTestBackend,
};
use ratatui::Terminal;
use std::{fs, hint::black_box, io, path::Path, process::Command, time::Duration};
use tempfile::TempDir;

fn git(repo: &Path, args: &[&str]) -> io::Result<()> {
    let output = Command::new("git")
        .args([
            "-c",
            "user.name=Benchmark",
            "-c",
            "user.email=bench@example.invalid",
        ])
        .args(args)
        .current_dir(repo)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", repo.join(".git/absent-user-config"))
        .env_remove("GIT_DIR")
        .env_remove("GIT_COMMON_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(io::Error::other(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ))
    }
}

fn seed_repo(root: &Path, directory: &Path) -> io::Result<()> {
    git(root, &["init", "--initial-branch=topic"])?;
    fs::create_dir(directory)?;
    for index in 0..256 {
        fs::write(
            directory.join(format!("document-{index:04}.txt")),
            b"entry\n",
        )?;
    }
    git(root, &["add", "documents"])?;
    git(root, &["commit", "-m", "Prepare directory listing"])
}

#[expect(
    clippy::expect_used,
    reason = "an unusable fixture must stop the benchmark"
)]
fn bench_confirmation(c: &mut Criterion, name: &str, command: MenuCommand, directory: &Path) {
    let panel = Panel::from_entries(directory.to_owned(), Vec::new());
    let mut app = App::with_panels(panel.clone(), panel);
    let mut terminal = Terminal::new(WritableTestBackend::new(80, 24)).expect("test terminal");
    let mut menu = Menu::open(Heading::Commands);
    menu.row = menu
        .entries()
        .iter()
        .position(|row| row.action() == Some(Action::Menu(command)))
        .expect("maintenance command in the Commands menu");
    let mut open = |app: &mut App| {
        app.set_menu(menu);
        dispatch_key(
            &mut terminal,
            app,
            KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        );
    };
    open(&mut app);
    assert!(is_confirmation(&app.overlay, command));
    c.bench_function(name, |b| {
        b.iter(|| {
            open(&mut app);
            black_box(&app.overlay);
        });
    });
}

fn is_confirmation(overlay: &Overlay, command: MenuCommand) -> bool {
    use ChoiceTarget::{Close, RunMaintenance};
    use MaintenanceCommand::{SyncBranch, Volume};
    use VolumeOperation::{Defragment, Trim};

    let Overlay::MenuDialog(dialog) = overlay else {
        return false;
    };
    let MenuDialog::Choices {
        rows, selected: 0, ..
    } = dialog.as_ref()
    else {
        return false;
    };
    let targets: Vec<_> = rows.iter().map(|row| &row.target).collect();
    match command {
        MenuCommand::SyncGitBranch => {
            matches!(targets.as_slice(), [Close, RunMaintenance(SyncBranch(_))])
        }
        MenuCommand::VolumeMaintenance => matches!(
            targets.as_slice(),
            [
                Close,
                RunMaintenance(Volume(Trim, _)),
                RunMaintenance(Volume(Defragment, _))
            ]
        ),
        _ => false,
    }
}

#[expect(
    clippy::expect_used,
    reason = "an unusable fixture must stop the benchmark"
)]
fn bench_maintenance(c: &mut Criterion) {
    let root = TempDir::new().expect("temporary benchmark workspace");
    let directory = root.path().join("documents");
    seed_repo(root.path(), &directory).expect("clean local repository");
    bench_confirmation(
        c,
        "maintenance/git_sync_confirm_256_files",
        MenuCommand::SyncGitBranch,
        &directory,
    );
    #[cfg(windows)]
    bench_confirmation(
        c,
        "maintenance/volume_confirm",
        MenuCommand::VolumeMaintenance,
        &directory,
    );
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10).warm_up_time(Duration::from_millis(250)).measurement_time(Duration::from_secs(2));
    targets = bench_maintenance
}
criterion_main!(benches);
