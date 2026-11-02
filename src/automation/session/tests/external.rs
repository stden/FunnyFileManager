//! Purpose: A running external program accepts input and resize while the app job is busy.
use super::*;
use std::thread;

#[test]
fn transport_input_and_resize_bypass_the_blocked_application_queue() {
    let (session, receiver) = queued_session();
    session.shared.reserve().unwrap();
    let host = session.host.clone();
    let child = thread::spawn(move || {
        let _scope = host.enter();
        crate::terminal::headless::run_with_timeout(
            std::process::Command::new("sh").args([
                "-c",
                "stty -echo; printf READY; IFS= read -r line; printf '<%s>' \"$line\"",
            ]),
            Some(Duration::from_secs(5)),
        )
    });
    let deadline = Instant::now() + Duration::from_secs(3);
    while !session.host.snapshot().output.contains("READY") && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(5));
    }
    assert!(session.host.snapshot().output.contains("READY"));
    let resized = session.resize(99, 31).unwrap();
    assert_eq!(resized["state"]["external_process"]["width"], 99);
    assert_eq!(resized["state"]["external_process"]["height"], 31);
    session
        .input(vec![], Some("private input\n".into()), None)
        .unwrap();
    assert_eq!(child.join().unwrap().unwrap().code, 0);
    assert!(session.host.snapshot().output.contains("<private input>"));
    assert!(receiver.try_recv().is_err());
    assert_eq!(session.state().unwrap()["busy"], true);
}
