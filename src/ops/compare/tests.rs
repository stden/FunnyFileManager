use crate::models::App;

#[test]
fn run_reports_that_the_row_is_coming() {
    let mut app = App::new().expect("app");
    super::run(&mut app);
    assert!(
        app.status_text().to_lowercase().contains("coming"),
        "{}",
        app.status_text()
    );
}
