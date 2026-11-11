use super::*;

fn connection(child: Child) -> Connection {
    let mut connection = Connection::from_io(tokio::io::empty(), tokio::io::sink());
    connection.child = Some(child);
    connection
}

fn hung_child() -> Child {
    #[cfg(unix)]
    {
        tokio::process::Command::new("sleep")
            .arg("60")
            .spawn()
            .unwrap()
    }
    #[cfg(windows)]
    {
        tokio::process::Command::new("ping.exe")
            .args(["-n", "60", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap()
    }
}

#[tokio::test(start_paused = true)]
async fn reaping_a_hung_child_kills_and_waits_for_it() {
    let mut connection = connection(hung_child());
    connection.reap().await;
    let child = connection.child.as_mut().unwrap();
    assert!(child.wait().await.is_ok());
    drop(connection);
}

#[test]
fn log_kill_outcome_tolerates_already_stopped_error() {
    log_kill_outcome(Ok(()));
    log_kill_outcome(Err(std::io::Error::other("already exited")));
}

#[tokio::test]
async fn reaping_an_exiting_child_returns_without_killing_it() {
    #[cfg(unix)]
    let child = tokio::process::Command::new("true").spawn().unwrap();
    #[cfg(windows)]
    let child = tokio::process::Command::new("cmd.exe")
        .args(["/C", "exit", "0"])
        .spawn()
        .unwrap();
    let mut connection = connection(child);
    connection.reap().await;
    drop(connection);
}

#[cfg(unix)]
#[tokio::test]
async fn reaping_an_externally_reaped_child_tolerates_the_kernel_error() {
    use rustix::process::{waitpid, Pid, WaitOptions};
    let child = tokio::process::Command::new("true").spawn().unwrap();
    let pid = Pid::from_raw(i32::try_from(child.id().unwrap()).unwrap()).unwrap();
    assert!(waitpid(Some(pid), WaitOptions::empty()).unwrap().is_some());
    let mut connection = connection(child);
    connection.reap().await;
    assert!(connection.child.as_mut().unwrap().try_wait().is_err());
    drop(connection);
}
