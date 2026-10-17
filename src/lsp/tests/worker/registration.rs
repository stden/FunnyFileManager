//! Purpose: Keep servers that register configuration notifications alive through diagnostics and queries.

use super::*;

#[test]
fn configuration_registration_does_not_disconnect_the_server_after_initialization() {
    let (root, settings, snapshot) = fixture("configuration-registration");
    let manager = Manager::start().unwrap();
    send(
        &manager,
        Command::Open("bat".into(), settings, snapshot.clone()),
    );
    let event = wait(&manager, |event| {
        matches!(
            event,
            Event::Diagnostics(_, _) | Event::State(ServerState::Failed)
        )
    });
    assert!(matches!(event, Event::Diagnostics(_, _)));
    send(
        &manager,
        Command::Query(Query {
            stamp: snapshot.stamp,
            position: Position::default(),
            feature: Feature::Hover,
        }),
    );
    wait(&manager, |event| matches!(event, Event::Reply(_, Ok(_))));
    drop(manager);
    assert!(fs::read_to_string(root.path().join("methods"))
        .unwrap()
        .ends_with("exit\n"));
}
