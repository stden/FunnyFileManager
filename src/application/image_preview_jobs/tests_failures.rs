//! A failed worker must not leave the viewer in a permanent loading state.

use super::*;
use std::time::{Duration, Instant};

#[test]
fn worker_panic_is_reported_to_the_open_viewer() {
    let mut app = App::new().expect("app");
    app.set_overlay(Overlay::Viewer(crate::models::Viewer::image(
        "broken.png".into(),
    )));
    let mut jobs = ImagePreviewJobs::with_decoder(|_| panic!("worker failed"));
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        jobs.pump(&mut app);
        if let Overlay::Viewer(viewer) = &app.overlay {
            if viewer.image == Some(ImageState::Failed("Image worker stopped".into())) {
                break;
            }
        }
        assert!(Instant::now() < until, "worker failure was not reported");
        std::thread::yield_now();
    }
}
