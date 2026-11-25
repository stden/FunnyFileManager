//! Unit tests: patch classification, HA twin wave separation, and GraphQL payload building.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::Duration;

use super::machine;
use crate::things::{
    build_plan, patch::build_start_payload, Machine, OsInfo, PatchKind, PatchScope, Status,
};

pub(super) fn os_machine(name: &str, os_name: &str, status: Status) -> Machine {
    Machine {
        os_info: Some(OsInfo {
            name: os_name.to_owned(),
            version: String::new(),
        }),
        status,
        ..machine(name, &format!("{name}.example.com"))
    }
}

pub(super) fn spawn_json_server(
    replies: Vec<serde_json::Value>,
) -> (String, std::thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("listener");
    let addr = listener.local_addr().expect("addr");
    let handle = std::thread::spawn(move || {
        let mut bodies = Vec::with_capacity(replies.len());
        for reply in replies {
            let (mut stream, _) = listener.accept().expect("accept");
            drop(stream.set_read_timeout(Some(Duration::from_secs(3))));
            let mut buf = [0_u8; 4096];
            let read = stream.read(&mut buf).unwrap_or(0);
            bodies.push(String::from_utf8_lossy(&buf[..read]).into_owned());
            let json_bytes = serde_json::to_vec(&reply).expect("json");
            let head = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                json_bytes.len()
            );
            stream.write_all(head.as_bytes()).expect("head");
            stream.write_all(&json_bytes).expect("body");
        }
        bodies
    });
    (format!("http://{addr}/graphql"), handle)
}

fn sample_machines() -> Vec<Machine> {
    vec![
        os_machine("srv-02", "Red Hat Enterprise Linux 9.8", Status::Online),
        os_machine("srv-01", "Red Hat Enterprise Linux 9.8", Status::Online),
        os_machine("win-01", "Windows Server 2025", Status::Online),
        os_machine("off-01", "Red Hat Enterprise Linux 9.8", Status::Offline),
        os_machine("sol-01", "Solaris 11", Status::Online),
        Machine {
            id: String::new(),
            name: "missing-id".to_owned(),
            ..os_machine("missing-id", "Linux", Status::Online)
        },
        Machine {
            os_info: None,
            tags: vec!["RHEL".to_owned()],
            ..machine("tag-lin", "tag-lin.example.com")
        },
        Machine {
            os_info: None,
            tags: vec!["Windows".to_owned()],
            ..machine("tag-win", "tag-win.example.com")
        },
    ]
}

#[test]
fn build_plan_separates_ha_twins_and_skips_unpatchable_machines() {
    let machines = sample_machines();
    let plan = build_plan(&machines, PatchScope::All, true, "monthly");
    assert_eq!(plan.target_count(), 5);
    assert_eq!(plan.waves[0].kind, PatchKind::Rhel);
    assert_eq!(plan.skipped.len(), 3);
    assert_eq!(
        build_plan(&machines, PatchScope::Rhel, false, "rhel").target_count(),
        3
    );
    assert!(PatchScope::parse_str("windows")
        .expect("scope")
        .allows(PatchKind::Windows));
    assert_eq!(PatchScope::parse_str("all"), Some(PatchScope::All));
    assert_eq!(PatchScope::parse_str(""), Some(PatchScope::All));
    assert!(PatchScope::parse_str("invalid").is_none());
    assert_eq!(PatchKind::Rhel.workflow_uri(), "patch_vm_rhel");
    assert_eq!(PatchKind::Windows.workflow_uri(), "patch_vm_windows");
    assert_eq!(PatchKind::Windows.label(), "Windows");
}

#[test]
fn numeric_only_hostnames_are_not_grouped_as_ha_twins() {
    let first = os_machine("01", "Red Hat Enterprise Linux 9.8", Status::Online);
    let second = os_machine("02", "Red Hat Enterprise Linux 9.8", Status::Online);
    let plan = build_plan(&[first, second], PatchScope::All, true, "numeric");
    assert_eq!(plan.waves.len(), 1);
    assert_eq!(plan.waves[0].targets.len(), 2);
}

#[test]
fn start_payload_sets_reboot_system_for_rhel_and_omits_it_for_windows() {
    let rhel = os_machine("web-01", "Red Hat Enterprise Linux 9.8", Status::Online);
    let win = os_machine("win-01", "Windows Server 2025", Status::Online);
    let plan = build_plan(&[rhel, win], PatchScope::All, true, "patch comment");
    let rhel_body = build_start_payload(&plan.waves[0], true, &plan.comment);
    assert_eq!(
        rhel_body["variables"]["input"]["typeNameUri"],
        "patch_vm_rhel"
    );
    assert_eq!(
        rhel_body["variables"]["input"]["inputs"][0]["rebootSystem"],
        true
    );
    let win_body = build_start_payload(&plan.waves[1], true, &plan.comment);
    assert_eq!(
        win_body["variables"]["input"]["typeNameUri"],
        "patch_vm_windows"
    );
    assert!(win_body["variables"]["input"]["inputs"][0]
        .get("rebootSystem")
        .is_none());
}
