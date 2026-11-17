//! Fixture unit tests for Linux `/proc/net/dev`, `/proc/net/route`,
//! `/sys/class/net` and `/etc/resolv.conf` network readers.

use std::fs;
use std::path::Path;
use std::time::Duration;

use crate::models::tasks::{AdapterKind, InterfaceState, Ipv4};

use super::files::Buffer;
use super::net::{parse_default_gateway, parse_resolv_conf, read_linux_network};

const NET_DEV_FIXTURE: &str = "\
Inter-|   Receive                                                |  Transmit
 face |bytes    packets errs drop fifo frame compressed multicast|bytes    packets errs drop fifo colls carrier compressed
   lo:    5000      50    0    0    0     0          0         0     5000      50    0    0    0     0       0          0
 eth0: 1200000    1000    1    2    0     0          0         0   400000     500    0    3    0     0       0          0
wlan0:   80000     200    0    0    0     0          0         0    20000     100    0    0    0     0       0          0
";

const NET_ROUTE_FIXTURE: &str = "\
Iface\tDestination\tGateway\tFlags\tRefCnt\tUse\tMetric\tMask\tMTU\tWindow\tIRTT
eth0\t0001A8C0\t00000000\t0001\t0\t0\t100\t00FFFFFF\t0\t0\t0
eth0\t00000000\t0101A8C0\t0003\t0\t0\t100\t00000000\t0\t0\t0
";

fn write_linux_net_tree(root: &Path) {
    let proc_root = root.join("proc");
    let sys_net = root.join("sys_net");
    fs::create_dir_all(proc_root.join("net")).unwrap();
    fs::create_dir_all(sys_net.join("eth0")).unwrap();
    fs::create_dir_all(sys_net.join("wlan0/wireless")).unwrap();
    fs::write(proc_root.join("net/dev"), NET_DEV_FIXTURE).unwrap();
    fs::write(proc_root.join("net/route"), NET_ROUTE_FIXTURE).unwrap();
    fs::write(sys_net.join("eth0/operstate"), "up\n").unwrap();
    fs::write(sys_net.join("eth0/speed"), "1000\n").unwrap();
    fs::write(sys_net.join("wlan0/operstate"), "down\n").unwrap();
    fs::write(
        root.join("resolv.conf"),
        "# Generated\nnameserver 192.168.1.1\nnameserver ::1\nnameserver 192.0.2.53 # backup\n",
    )
    .unwrap();
}

#[test]
fn linux_network_reader_parses_proc_net_dev_sysfs_route_and_resolv_conf() {
    let dir = tempfile::TempDir::new().expect("tempdir");
    let root = dir.path();
    write_linux_net_tree(root);
    let mut buf = Buffer::default();
    let sample = read_linux_network(
        &root.join("proc"),
        &root.join("sys_net"),
        &root.join("resolv.conf"),
        Duration::from_secs(5),
        &mut buf,
    );
    assert_eq!(sample.gateway, Some(Ipv4::new(192, 168, 1, 1)));
    assert_eq!(sample.dns_servers.len(), 2);
    assert_eq!(sample.adapters.len(), 3);
    assert_eq!(sample.adapters[0].kind, AdapterKind::Loopback);
    assert_eq!(sample.adapters[1].kind, AdapterKind::Ethernet);
    assert_eq!(sample.adapters[1].link_bps, Some(1_000_000_000));
    assert_eq!(sample.adapters[1].rx_bytes, 1_200_000);
    assert_eq!(sample.adapters[1].tx_drops, 3);
    assert_eq!(sample.adapters[2].kind, AdapterKind::Wifi);
    assert_eq!(sample.adapters[2].state, InterfaceState::Down);

    assert_eq!(parse_default_gateway("header only\n"), None);
    assert_eq!(parse_resolv_conf("search local\n").len(), 0);
}
