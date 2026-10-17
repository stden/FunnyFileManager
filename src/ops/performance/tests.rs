use super::*;

#[test]
fn opens_a_report_without_requiring_a_host_terminal() {
    let temp = tempfile::TempDir::new().expect("tempdir");
    let panel = crate::models::Panel::new(temp.path().to_path_buf()).expect("panel");
    let mut app = App::with_panels(panel.clone(), panel);
    open(&mut app);
    assert!(matches!(app.overlay, Overlay::Viewer(_)));
}
