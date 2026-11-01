//! Purpose: Network mounts share YAML with server profiles and user settings.
use super::*;
use crate::models::network_drive::{MountPoint, NetworkAddress, NetworkDrive, NetworkProtocol};

fn drive() -> NetworkDrive {
    NetworkDrive {
        path: MountPoint::try_from("Z:\\".to_owned()).expect("mount"),
        remote: NetworkAddress::try_from("\\\\example.test\\share".to_owned()).expect("remote"),
        protocol: NetworkProtocol::Smb,
    }
}
#[test]
fn network_drive_snapshot_preserves_profiles_and_remembers_offline_mounts() {
    let temp = tempfile::TempDir::new().expect("temp");
    let path = temp.path().join("ffm.yml");
    std::fs::write(&path, "sessions: []\ncustom: true\n").expect("initial");
    save_to(&path, &[drive()]).expect("save");
    save_to(&path, &[]).expect("offline refresh");
    let text = std::fs::read_to_string(path).expect("read");
    let document: serde_norway::Value = serde_norway::from_str(&text).expect("YAML");
    assert_eq!(document["custom"].as_bool(), Some(true));
    assert_eq!(
        document["sessions"].as_sequence().expect("profiles").len(),
        0
    );
    assert_eq!(
        document["network_drives"]
            .as_sequence()
            .expect("drives")
            .len(),
        1
    );
}
#[test]
fn network_mount_types_reject_relative_paths_and_control_characters() {
    assert!(MountPoint::try_from("relative".to_owned()).is_err());
    assert!(NetworkAddress::try_from("host\nshare".to_owned()).is_err());
    assert!(NetworkAddress::try_from("https://user:password@example.test/dav".to_owned()).is_err());
}

#[test]
fn linux_network_mounts_are_decoded_and_local_filesystems_are_excluded() {
    let mounts = discover::parse_mounts("server:/export /mnt/nfs nfs4 rw 0 0\n//server/share /mnt/team\\040files cifs rw 0 0\nuser@server:/work /mnt/ssh fuse.sshfs rw 0 0\nhttps://server/dav /mnt/dav davfs rw 0 0\n/dev/root / ext4 rw 0 0\n").expect("mounts");
    assert_eq!(mounts.len(), 4);
    assert_eq!(String::from(mounts[1].path.clone()), "/mnt/team files");
}

#[test]
fn discovering_drives_before_profile_save_preserves_legacy_sessions() {
    let temp = tempfile::TempDir::new().expect("temp");
    let mut store = crate::sessions::SessionStore::load_from(temp.path().join("sessions.json"));
    store.upsert(crate::models::Session::new(
        crate::models::Protocol::Sftp,
        crate::models::HostName::new("example.test").expect("host"),
        crate::models::Port::SSH,
        None,
    ));
    store.save().expect("legacy");
    let path = temp.path().join("ffm.yml");
    save_to(&path, &[drive()]).expect("discover drives");
    assert_eq!(crate::sessions::SessionStore::load_from(path).len(), 1);
}
