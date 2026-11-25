//! Purpose: Drive queued remote operations synchronously in state-transition tests.

use crate::models::App;

pub fn finish_remote(app: &mut App) {
    while let Some(request) = app.remote_requests.take() {
        let answer = crate::ops::remote::jobs::execute(&request.task);
        assert!(app.remote_requests.finish(request.id));
        crate::ops::remote::completion::apply(app, request, answer);
    }
}
