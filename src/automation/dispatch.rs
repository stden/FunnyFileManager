//! Purpose: Route transport requests to the application session's typed API.
use super::{catalog, protocol::Request, session::Session};
use serde_json::{json, Value};
use std::io;

pub(super) fn request(session: &mut Session, request: Request) -> io::Result<Value> {
    match request {
        Request::Catalog {} => {
            Ok(json!({"actions": catalog::entries(), "state": session.state()?}))
        }
        Request::State {} => session.response(),
        Request::Action { context, action } => session.action(&context, &action),
        Request::Input { keys, text, mouse } => session.input(keys, text, mouse),
        Request::Wait { timeout_ms } => session.wait(timeout_ms),
        Request::Resize { width, height } => session.resize(width, height),
        Request::Cancel {} => session.cancel(),
        Request::Close {} => session.close(),
    }
}
