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

#[test]
fn duplicate_pipe_preconnected_accept_missing_server_and_non_pipe_handles_are_checked() {
    let name = format!("ffm-test-edge-{}", std::process::id());
    let server = super::create_pipe(&name).expect("first pipe");
    assert!(super::create_pipe(&name).is_err());

    let client = super::open_pipe(&name, Duration::from_millis(200)).expect("pre-connect");
    super::accept(&server).expect("already connected pipe succeeds");
    drop((client, server));

    assert!(super::open_pipe("ffm-no-such-pipe-ever", Duration::from_millis(60)).is_err());

    let regular = tempfile::tempfile().expect("regular file");
    assert!(super::accept(&regular).is_err());
    assert!(super::client_process_id(&regular).is_err());
    assert!(super::server_process_id(&regular).is_err());
}
