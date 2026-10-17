//! Purpose: The local pipe between FFM and the helper it started, on Windows.

use std::time::Duration;

#[test]
fn a_client_and_the_server_are_this_process() {
    let name = format!("ffm-test-{}", std::process::id());
    let server = super::create_pipe(&name).expect("pipe");
    let client_name = name;
    let client = std::thread::spawn(move || {
        super::open_pipe(&client_name, Duration::from_secs(2)).expect("client")
    });
    super::accept(&server).expect("accept");
    let client = client.join().expect("joined");
    let pid = std::process::id();
    assert_eq!(super::client_process_id(&server).expect("client id"), pid);
    assert_eq!(super::server_process_id(&client).expect("server id"), pid);
    let _is_elevated = super::super::is_elevated();
}
