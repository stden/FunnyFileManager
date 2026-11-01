//! Purpose: Input validation is atomic and named actions honor modal context ownership.

use super::*;

#[test]
fn an_invalid_key_rejects_the_entire_batch_before_opening_or_editing_a_file() {
    let root = workspace();
    let mut session = Session::start(root.path());
    session.enter("left");
    session.select("file.txt");
    let response = session.request(json!({"operation":"input",
        "keys":["F4","DefinitelyNotAKey"],"text":"must-not-be-written"}));
    assert_eq!(response["ok"], false);
    let state = session.state();
    assert_eq!(state["modal"]["kind"], "none");
    assert_eq!(state["modal"]["context"], "panels");
    assert_eq!(
        fs::read(root.path().join("left/file.txt")).unwrap(),
        b"alpha\n"
    );
    session.close();
}

#[test]
fn unknown_input_fields_are_rejected_before_any_key_runs() {
    let root = workspace();
    let mut session = Session::start(root.path());
    let response = session.request(json!({"operation":"input","keys":["F1"],"unexpected":true}));
    assert_eq!(response["ok"], false);
    assert_eq!(session.state()["modal"]["kind"], "none");
    session.close();
}

#[test]
fn document_actions_refuse_the_panels_context_without_changing_files() {
    let root = workspace();
    let mut session = Session::start(root.path());
    session.enter("left");
    let response =
        session.request(json!({"operation":"action","context":"editor","action":"newline"}));
    let state = if response["ok"] == true {
        let state = session.settle(response["result"]["state"].clone());
        assert!(!state["job"]["error"].is_null());
        state
    } else {
        let state = session.request(json!({"operation":"state"}));
        state["result"]["state"].clone()
    };
    assert_eq!(state["modal"]["kind"], "none");
    assert_eq!(
        fs::read(root.path().join("left/file.txt")).unwrap(),
        b"alpha\n"
    );
    session.close();
}

#[test]
fn resizing_and_mouse_input_affect_the_real_rendered_panel_state() {
    let root = workspace();
    let mut session = Session::start(root.path());
    assert_eq!(
        session.state()["screen"]["rows"].as_array().unwrap().len(),
        24
    );
    let state = session.perform(json!({"operation":"resize","width":100,"height":30}));
    assert_eq!(state["screen"]["width"], 100);
    assert_eq!(state["screen"]["height"], 30);
    assert_eq!(state["screen"]["rows"].as_array().unwrap().len(), 30);
    assert!(screen(&state).contains("left"));
    let state = session.perform(json!({"operation":"input","mouse":{
        "kind":"down","button":"left","column":75,"row":5,"modifiers":[]}}));
    assert_eq!(state["panels"]["active"], "right");
    assert_path(&state["panels"]["left"]["path"], root.path());
    session.close();
}
