//! Purpose: Deliver a database answer to the explorer that asked for it. An
//! answer for a dialog that is no longer open is dropped by the explorer, which
//! no longer waits for its request.

use super::DbError;
use crate::models::database::{Reply, RequestId};
use crate::models::{App, Overlay};
use crate::status::Status;

pub(crate) fn apply(app: &mut App, id: RequestId, answer: Result<Reply, DbError>) {
    let message = match &mut app.overlay {
        Overlay::Database(explorer) => {
            explorer.receive(id, answer.map_err(|error| error.to_string()))
        }
        _ => None,
    };
    if let Some(text) = message {
        app.set_status(Status::Error(text));
    }
}
