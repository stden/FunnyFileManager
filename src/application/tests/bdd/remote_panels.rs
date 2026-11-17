//! Purpose: BDD steps for SFTP and FTP remote file panels over `FakeRemoteFs`.

use cucumber::{gherkin::Step, given, then, when};

use super::BehaviorWorld;
use crate::{
    models::{HostName, PanelKind, Protocol, RemotePath, RowMotion, Session, UserName},
    remote::{fake::FakeRemoteFs, RemoteFs, WriteMode},
};

#[given(expr = "the left panel is connected to an {string} host with remote files:")]
fn connect_root_host(world: &mut BehaviorWorld, step: &Step, protocol: String) {
    connect_host(world, step, &protocol, None);
}

#[given(expr = "the left panel is connected to an {string} host at {string} with remote files:")]
fn connect_subdir_host(world: &mut BehaviorWorld, step: &Step, protocol: String, start: String) {
    connect_host(world, step, &protocol, Some(start.as_str()));
}

fn connect_host(world: &mut BehaviorWorld, step: &Step, protocol: &str, start: Option<&str>) {
    let proto = parse_protocol(protocol);
    let mut session = Session::new(
        proto,
        HostName::new("remote.example.test").unwrap(),
        proto.default_port(),
        Some(UserName::new("ops").unwrap()),
    );
    session.remote_dir = start.map(|dir| RemotePath::parse(dir).unwrap());
    let fs = build_fake_fs(step);
    let case = world.case();
    case.app.sessions.upsert(session.clone());
    crate::ops::remote::open_with(&mut case.app, &session, Box::new(fs)).unwrap();
    case.paint();
}

fn parse_protocol(protocol: &str) -> Protocol {
    match protocol {
        "SFTP" => Protocol::Sftp,
        "FTP" => Protocol::Ftp,
        _ => panic!("unsupported test protocol {protocol:?}"),
    }
}

fn build_fake_fs(step: &Step) -> FakeRemoteFs {
    let rows = &step.table().expect("table of remote files").rows;
    let sizes: Vec<(&str, u64)> = rows
        .iter()
        .map(|row| (row[0].as_str(), u64::try_from(row[1].len()).unwrap()))
        .collect();
    let mut fs = FakeRemoteFs::with_files(&sizes);
    for row in rows {
        let path = RemotePath::parse(&row[0]).unwrap();
        let mut cursor = std::io::Cursor::new(row[1].as_bytes());
        fs.upload(&path, &mut cursor, WriteMode::Overwrite).unwrap();
    }
    fs
}

#[when(expr = "I select remote entry {string} and open it")]
fn select_and_open(world: &mut BehaviorWorld, target: String) {
    select_remote_entry(world, &target);
    let case = world.case();
    case.press("Enter");
    crate::test_support::finish_remote(&mut case.app);
    case.paint();
}

#[when(expr = "I select remote entry {string} and press {string} and finish remote work")]
fn select_press_and_finish(world: &mut BehaviorWorld, target: String, keys: String) {
    select_remote_entry(world, &target);
    let case = world.case();
    case.press(&keys);
    crate::test_support::finish_remote(&mut case.app);
    case.paint();
}

#[when(expr = "I select remote entry {string} and press {string}")]
fn select_and_press(world: &mut BehaviorWorld, target: String, keys: String) {
    select_remote_entry(world, &target);
    world.case().press(&keys);
}

#[when("remote work finishes")]
fn finish_remote_step(world: &mut BehaviorWorld) {
    let case = world.case();
    crate::test_support::finish_remote(&mut case.app);
    case.paint();
}

fn select_remote_entry(world: &mut BehaviorWorld, target: &str) {
    let PanelKind::Remote(panel) = &mut world.case().app.left_panel.kind else {
        panic!("expected left panel to browse a remote host");
    };
    let index = panel
        .entries()
        .iter()
        .position(|entry| entry.name == target)
        .unwrap_or_else(|| panic!("missing remote entry {target:?}"));
    panel.move_cursor(RowMotion::First);
    for _ in 0..index {
        panel.move_cursor(RowMotion::Next);
    }
}

#[then(expr = "the left remote directory is {string}")]
fn remote_directory_equals(world: &mut BehaviorWorld, expected: String) {
    let PanelKind::Remote(panel) = &world.case().app.left_panel.kind else {
        panic!("expected left panel to browse a remote host");
    };
    assert_eq!(panel.path().as_str(), expected);
}

#[then(expr = "the left remote listing equals {string}")]
fn remote_listing_equals(world: &mut BehaviorWorld, expected: String) {
    let PanelKind::Remote(panel) = &world.case().app.left_panel.kind else {
        panic!("expected left panel to browse a remote host");
    };
    let actual: Vec<&str> = panel.entries().iter().map(|e| e.name.as_str()).collect();
    let wanted: Vec<&str> = expected.split_whitespace().collect();
    assert_eq!(actual, wanted);
}

#[then("the left panel returns to local files")]
fn left_panel_is_local(world: &mut BehaviorWorld) {
    assert_eq!(world.case().app.left_panel.kind, PanelKind::Files);
}
