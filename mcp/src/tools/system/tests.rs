use super::*;

#[test]
fn build_system_info_handles_both_present_and_absent_host_fields() {
    let minimal = build_system_info(None, None, None, None);
    assert_eq!(minimal["os"], std::env::consts::OS);
    assert!(minimal.get("current_directory").is_none());
    assert!(minimal.get("home_directory").is_none());
    assert!(minimal.get("hostname").is_none());
    assert!(minimal.get("root_disk").is_none());

    let full = build_system_info(
        Some(Path::new("/work")),
        Some(Path::new("/home/user")),
        Some(" host.local \n"),
        Some("/dev/sda1 10G 5G 5G 50% /"),
    );
    assert_eq!(full["hostname"], "host.local");
    assert_eq!(full["root_disk"], "/dev/sda1 10G 5G 5G 50% /");
    assert!(full["current_directory"].is_string());
    assert!(full["home_directory"].is_string());
}
