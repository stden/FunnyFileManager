//! Purpose: `listed` ignores a panel target when the slot no longer holds a remote panel.

use crate::models::remote_work::RemoteTarget;
use crate::models::{PanelKind, PanelSide, RemotePath};
use crate::remote::fake::FakeRemoteFs;
use crate::remote::RemoteLink;
use crate::test_support::netbox::app_in;

#[test]
fn listed_ignores_panel_target_when_panel_is_no_longer_remote() {
    let dir = tempfile::TempDir::new().expect("temp dir");
    let mut app = app_in(dir.path());
    let link = RemoteLink::new(Box::new(FakeRemoteFs::with_files(&[])));
    let target = RemoteTarget::Panel {
        side: PanelSide::Left,
        link: link.clone(),
        path: RemotePath::root(),
    };

    super::listed(&mut app, &target, link, RemotePath::root(), Vec::new());

    assert_eq!(app.left_panel.kind, PanelKind::Files);
}
