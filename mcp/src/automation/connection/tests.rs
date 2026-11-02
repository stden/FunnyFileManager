use super::*;

#[cfg(unix)]
fn connection(child: Child) -> Connection {
    let mut connection = Connection::from_io(tokio::io::empty(), tokio::io::sink());
    connection.child = Some(child);
    connection
}

#[cfg(unix)]
#[tokio::test(start_paused = true)]
async fn reaping_a_hung_child_kills_and_waits_for_it() {
    let child = tokio::process::Command::new("sleep").arg("60").spawn().unwrap();
    let mut connection = connection(child);
    connection.reap().await;
    let child = connection.child.as_mut().unwrap();
    assert!(child.try_wait().unwrap().is_some());
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
