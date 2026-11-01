//! Purpose: A Things read answered through the job seam, and the machine listing
//! such a read returns.

use std::time::{Duration, Instant};

use serde_json::json;

use crate::{
    application::things_jobs::ThingsJobs,
    models::{things::ThingsState, App, Overlay},
    things::{self, Listing, ThingsError},
};

const READ_DEADLINE: Duration = Duration::from_secs(5);

/// Starts the queued read so that it answers `answer`, then publishes that answer
/// once its worker has sent it.
pub(super) fn settle(app: &mut App, jobs: &mut ThingsJobs, answer: Result<Listing, ThingsError>) {
    jobs.pump_with(app, move |_config| answer);
    let deadline = Instant::now() + READ_DEADLINE;
    while reading(app) {
        assert!(Instant::now() < deadline, "the Things read did not finish");
        jobs.pump_with(app, |_config| Err(ThingsError::Disabled));
        std::thread::yield_now();
    }
}

/// Whether a read is queued or out, so the dialog does not show a catalog yet.
fn reading(app: &App) -> bool {
    matches!(
        &app.overlay,
        Overlay::Things(dialog)
            if matches!(dialog.state, ThingsState::Queued(_) | ThingsState::Loading)
    )
}

/// The listing a read returns for `names`: one online machine per name, with the
/// host `NAME-host.example.test`.
pub(super) fn machines(names: &str) -> Listing {
    let body: Vec<_> = names
        .split_whitespace()
        .map(|name| {
            json!({
                "id": name,
                "name": name,
                "hostname": format!("{name}-host.example.test"),
                "status": "online",
            })
        })
        .collect();
    things::parse(&serde_json::to_vec(&body).unwrap()).unwrap()
}
