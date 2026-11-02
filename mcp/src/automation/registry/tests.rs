use super::*;

async fn session(connection: Option<Connection>) -> Arc<Session> {
    Arc::new(Session {
        connection: Mutex::new(connection),
        _permit: Arc::new(Semaphore::new(1)).acquire_owned().await.unwrap(),
    })
}

#[test]
fn debugging_does_not_expose_session_contents_or_launch_configuration() {
    assert_eq!(
        format!("{:?}", SessionManager::default()),
        "SessionManager { .. }"
    );
}

#[tokio::test]
async fn requests_to_a_retired_connection_return_unknown_session() {
    let session = session(None).await;
    assert!(matches!(
        session.request(&Request::State).await,
        Err(Error::UnknownSession)
    ));
    drop(session);
}

#[tokio::test]
async fn closing_a_retired_connection_still_forgets_its_session_id() {
    let manager = SessionManager::default();
    let id = SessionId("retired-session".into());
    manager.sessions.lock().await.insert(id.clone(), session(None).await);
    assert!(matches!(manager.close(&id).await, Err(Error::UnknownSession)));
    assert!(manager.sessions.lock().await.is_empty());
    drop(manager);
}

#[tokio::test(start_paused = true)]
async fn a_busy_connection_returns_a_bounded_application_refusal() {
    let session = session(None).await;
    let guard = session.connection.lock().await;
    let error = session.request(&Request::State).await.unwrap_err();
    drop(guard);
    drop(session);
    assert!(
        matches!(error, Error::Application { code, message } if code == "busy" && message.contains("still running"))
    );
}

#[tokio::test(start_paused = true)]
async fn a_missing_response_times_out_and_retires_the_connection() {
    let (client, peer) = tokio::io::duplex(4096);
    let (read, write) = tokio::io::split(client);
    let session = session(Some(Connection::from_io(read, write))).await;
    assert!(matches!(
        session.request(&Request::State).await,
        Err(Error::Timeout)
    ));
    assert!(session.connection.lock().await.is_none());
    drop(session);
    drop(peer);
}

#[tokio::test(start_paused = true)]
async fn closing_a_busy_session_removes_its_id_even_if_lock_cleanup_times_out() {
    let manager = SessionManager::default();
    let id = SessionId("busy-session".into());
    let session = session(None).await;
    manager
        .sessions
        .lock()
        .await
        .insert(id.clone(), Arc::clone(&session));
    let guard = session.connection.lock().await;
    let error = manager.close(&id).await.unwrap_err();
    drop(guard);
    drop(session);
    assert!(matches!(error, Error::Timeout));
    assert!(manager.sessions.lock().await.is_empty());
    drop(manager);
}

#[derive(Default)]
struct SilentLauncher(std::sync::Mutex<Vec<tokio::io::DuplexStream>>);

impl launch::Launcher for SilentLauncher {
    fn launch(&self, _: &Path, _: Size, _: Profile) -> Result<Connection, Error> {
        let (client, peer) = tokio::io::duplex(4096);
        self.0.lock().unwrap().push(peer);
        let (read, write) = tokio::io::split(client);
        Ok(Connection::from_io(read, write))
    }
}

#[tokio::test(start_paused = true)]
async fn an_absent_handshake_releases_the_reserved_slot_after_its_deadline() {
    let manager = SessionManager::with_launcher(Arc::new(SilentLauncher::default()));
    let root = tempfile::tempdir().unwrap();
    let result = manager
        .start(root.path(), Size::new(80, 24).unwrap(), Profile::Isolated)
        .await;
    assert!(matches!(result, Err(Error::Timeout)));
    assert!(manager.sessions.lock().await.is_empty());
    assert_eq!(manager.capacity.available_permits(), 4);
    drop(manager);
}
