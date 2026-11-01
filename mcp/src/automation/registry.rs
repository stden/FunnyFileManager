//! Purpose: Share session ownership across server clones and retire broken children.

use super::{connection::Connection, launch, Error, Profile, Request, SessionId, Size, Started};
use serde_json::Value;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::Path;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};
use tokio::time::{timeout, timeout_at, Instant};

const RESPONSE_TIMEOUT: Duration = Duration::from_secs(15);

pub struct SessionManager {
    pub(super) sessions: Mutex<BTreeMap<SessionId, Arc<Session>>>,
    launcher: Arc<dyn launch::Launcher>,
    capacity: Arc<Semaphore>,
    next_id: AtomicU64,
}

pub(super) struct Session {
    connection: Mutex<Option<Connection>>,
    _permit: OwnedSemaphorePermit,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::with_launcher(Arc::new(launch::BinaryLauncher(OsString::from("ffm"))))
    }
}

impl std::fmt::Debug for SessionManager {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SessionManager")
            .finish_non_exhaustive()
    }
}

impl SessionManager {
    pub(crate) fn from_environment() -> Self {
        let binary =
            std::env::var_os("FFM_MCP_FFM_BINARY").unwrap_or_else(|| OsString::from("ffm"));
        Self::with_launcher(Arc::new(launch::BinaryLauncher(binary)))
    }

    pub(super) fn with_launcher(launcher: Arc<dyn launch::Launcher>) -> Self {
        Self {
            sessions: Mutex::new(BTreeMap::new()),
            launcher,
            capacity: Arc::new(Semaphore::new(4)),
            next_id: AtomicU64::new(1),
        }
    }

    #[expect(
        clippy::significant_drop_tightening,
        reason = "the permit reserves capacity throughout startup and is transferred into the session"
    )]
    pub(crate) async fn start(
        &self,
        root: &Path,
        size: Size,
        profile: Profile,
    ) -> Result<Started, Error> {
        let permit = Arc::clone(&self.capacity)
            .try_acquire_owned()
            .map_err(|_capacity| Error::SessionLimit)?;
        let mut connection = self.launcher.launch(root, size, profile)?;
        let state = timeout(Duration::from_secs(5), connection.handshake())
            .await
            .map_err(|_deadline| Error::Timeout)??;
        let session_id = self.next_session_id();
        let session = Session {
            connection: Mutex::new(Some(connection)),
            _permit: permit,
        };
        self.sessions
            .lock()
            .await
            .insert(session_id.clone(), Arc::new(session));
        Ok(Started {
            session_id,
            protocol_version: 1,
            state,
        })
    }

    fn next_session_id(&self) -> SessionId {
        SessionId(format!(
            "ffm-{}-{}",
            std::process::id(),
            self.next_id.fetch_add(1, Ordering::Relaxed)
        ))
    }

    pub(crate) async fn request(&self, id: &SessionId, request: Request) -> Result<Value, Error> {
        let session = self
            .sessions
            .lock()
            .await
            .get(id)
            .cloned()
            .ok_or(Error::UnknownSession)?;
        let result = session.request(&request).await;
        drop(session);
        if result.as_ref().is_err_and(Error::is_fatal) {
            self.sessions.lock().await.remove(id);
        }
        result
    }

    pub(crate) async fn close(&self, id: &SessionId) -> Result<Value, Error> {
        let session = self
            .sessions
            .lock()
            .await
            .remove(id)
            .ok_or(Error::UnknownSession)?;
        let result = session.request(&Request::Close).await;
        let mut guard = timeout(RESPONSE_TIMEOUT, session.connection.lock())
            .await
            .map_err(|_deadline| Error::Timeout)?;
        if let Some(mut connection) = guard.take() {
            connection.reap().await;
        }
        drop(guard);
        drop(session);
        result
    }
}

impl Session {
    async fn request(&self, request: &Request) -> Result<Value, Error> {
        let deadline = Instant::now() + RESPONSE_TIMEOUT;
        let mut guard =
            timeout_at(deadline, self.connection.lock())
                .await
                .map_err(|_deadline| Error::Application {
                    code: "busy".to_owned(),
                    message: "another session request is still running".to_owned(),
                })?;
        let connection = guard.as_mut().ok_or(Error::UnknownSession)?;
        let result = timeout_at(deadline, connection.exchange(request))
            .await
            .unwrap_or(Err(Error::Timeout));
        if result.as_ref().is_err_and(Error::is_fatal) {
            *guard = None;
        }
        drop(guard);
        result
    }
}
