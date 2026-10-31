//! Edges of the Linux network parsers: broken rows, gateway flags, and DNS.

use std::fs;
use std::path::Path;
use std::time::Duration;

use crate::models::tasks::{AdapterKind, AdapterSample, InterfaceState, Ipv4, NetworkSample};

use super::files::Buffer;
use super::net::{parse_default_gateway, parse_net_dev, parse_resolv_conf, read_linux_network};

const DEV: &str = "\
h
h
no-colon
   : 1 2 3 4 5 6 7 8 9 10 11 12
eth0: 1
eth0: nope 1 2 3 4 5 6 7 8 9 10 11
eth0: 10 1 2 3 0 0 0 0 20 4 5 6
enp3s0: 10 1 2 3 0 0 0 0 20 4 5 6
wlan9: 10 1 2 3 0 0 0 0 20 4 5 6
veth0: 10 1 2 3 0 0 0 0 20 4 5 6
mystery: 10 1 2 3 0 0 0 0 20 4 5 6
lo: 10 1 2 3 0 0 0 0 20 4 5 6
";

fn plant(root: &Path, name: &str, state: &str, speed: &str) {
    let dir = root.join(name);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("operstate"), state).unwrap();
    fs::write(dir.join("speed"), speed).unwrap();
}

fn plant_all(root: &Path) {
    plant(root, "eth0", "dormant", "0");
    plant(root, "enp3s0", "up", "1");
    plant(root, "wlan9", "down", "-1");
    plant(root, "veth0", "up", "4294967295");
    plant(root, "mystery", "weird", "100");
    plant(root, "lo", "down", "10");
}

fn adapters(sys: &Path, text: &str) -> Vec<AdapterSample> {
    let mut buffer = Buffer::default();
    parse_net_dev(text, sys, &mut buffer)
}

#[test]
fn malformed_net_dev_rows_are_skipped_and_a_full_row_keeps_its_counters() {
    let dir = tempfile::TempDir::new().expect("temp");
    plant_all(dir.path());
    let found = adapters(dir.path(), DEV);
    let names: Vec<_> = found.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(names, ["eth0", "enp3s0", "wlan9", "veth0", "mystery", "lo"]);
    let eth = &found[0];
    assert_eq!((eth.rx_bytes, eth.tx_bytes, eth.rx_drops), (10, 20, 3));
    assert_eq!(eth.tx_drops, 6);
    assert_eq!(eth.state, InterfaceState::Dormant);
    assert_eq!(eth.link_bps, None);
}

#[test]
fn names_and_sysfs_choose_kind_and_reject_unusable_speeds() {
    let dir = tempfile::TempDir::new().expect("temp");
    plant_all(dir.path());
    let found = adapters(dir.path(), DEV);
    assert_eq!(found[0].kind, AdapterKind::Ethernet);
    assert_eq!(found[1].kind, AdapterKind::Ethernet);
    assert_eq!(found[1].link_bps, Some(1_000_000));
    assert_eq!(found[2].kind, AdapterKind::Wifi);
    assert_eq!(found[2].state, InterfaceState::Down);
    assert_eq!(found[2].link_bps, None);
    assert_eq!(found[3].kind, AdapterKind::Virtual);
    assert_eq!(found[3].link_bps, None);
    assert_eq!(found[4].kind, AdapterKind::Other);
    assert_eq!(found[4].state, InterfaceState::Unknown);
    assert_eq!(found[4].link_bps, Some(100_000_000));
    assert_eq!(found[5].kind, AdapterKind::Loopback);
    assert_eq!(found[5].state, InterfaceState::Down);
}

#[test]
fn a_loopback_without_sysfs_is_up_and_other_names_stay_unknown() {
    let dir = tempfile::TempDir::new().expect("temp");
    let text = "h\nh\nlo: 1 1 0 0 0 0 0 0 1 1 0 0\nnone: 1 1 0 0 0 0 0 0 1 1 0 0\n";
    let found = adapters(dir.path(), text);
    assert_eq!(found[0].kind, AdapterKind::Loopback);
    assert_eq!(found[0].state, InterfaceState::Up);
    assert_eq!(found[1].kind, AdapterKind::Other);
    assert_eq!(found[1].state, InterfaceState::Unknown);
    assert_eq!(found[1].link_bps, None);
}

#[test]
fn missing_network_files_are_an_empty_sample() {
    let dir = tempfile::TempDir::new().expect("temp");
    let missing = dir.path().join("absent");
    let mut buffer = Buffer::default();
    let sample = read_linux_network(
        &missing,
        &missing,
        &missing,
        Duration::from_secs(3),
        &mut buffer,
    );
    assert_eq!(sample, NetworkSample::empty());
}

#[test]
fn gateway_flags_and_resolv_conf_drop_unusable_addresses() {
    assert_eq!(
        parse_default_gateway("h\neth0 00000000 0101A8C0 0001\n"),
        None
    );
    assert_eq!(
        parse_default_gateway("h\neth0 00000000 00000000 0003\n"),
        None
    );
    assert_eq!(parse_default_gateway("h\neth0 00000000 ZZZZ 0003\n"), None);
    assert_eq!(parse_default_gateway("h\nshort\n"), None);
    let dns = parse_resolv_conf(
        "nameserver 0.0.0.0\nnameserver 192.0.2.1\nnameserver 192.0.2.1\n\
         nameserver nope\n# nameserver 192.0.2.9\nnameserver 192.0.2.8 # note\n",
    );
    assert_eq!(dns, vec![Ipv4::new(192, 0, 2, 1), Ipv4::new(192, 0, 2, 8)]);
}
