//! Purpose: unit tests for the kill confirmation's buttons and the signal each sends.

use super::*;

#[cfg(unix)]
#[test]
fn terminate_or_kill_focuses_terminate_and_wraps() {
    let mut confirm = KillConfirm::new(vim(), KillOffer::TerminateOrKill);
    assert_eq!(confirm.targets().targets().len(), 1);
    assert_eq!(confirm.offer(), KillOffer::TerminateOrKill);
    assert_eq!(
        confirm.buttons(),
        [
            ConfirmButton::Terminate,
            ConfirmButton::Kill,
            ConfirmButton::Cancel
        ]
    );
    assert_eq!(confirm.focused(), ConfirmButton::Terminate);
    confirm.focus_next();
    assert_eq!(confirm.signal(confirm.focused()), Some(Signal::Kill));
    confirm.focus_next();
    confirm.focus_next();
    assert_eq!(confirm.focused(), ConfirmButton::Terminate, "wrapped");
    confirm.focus_previous();
    assert_eq!(confirm.focused(), ConfirmButton::Cancel);
    assert_eq!(confirm.signal(ConfirmButton::Cancel), None);
    assert_eq!(
        confirm.signal(ConfirmButton::Send),
        None,
        "not on this dialog"
    );
}

#[test]
fn a_chosen_signal_is_sent_by_the_send_button_only() {
    let mut confirm = KillConfirm::new(vim(), KillOffer::Send(Signal::Hangup));
    assert_eq!(
        confirm.buttons(),
        [ConfirmButton::Send, ConfirmButton::Cancel]
    );
    assert_eq!(confirm.signal(ConfirmButton::Send), Some(Signal::Hangup));
    assert_eq!(confirm.signal(ConfirmButton::Kill), None);
    confirm.focus_on(ConfirmButton::Kill);
    assert_eq!(
        confirm.focused(),
        ConfirmButton::Send,
        "a missing button is not focused"
    );
    confirm.focus_on(ConfirmButton::Cancel);
    assert_eq!(confirm.focused(), ConfirmButton::Cancel);
}

#[test]
fn terminate_or_kill_maps_buttons_and_wraps_on_every_platform() {
    let mut confirm = KillConfirm::new(vim(), KillOffer::TerminateOrKill);
    assert_eq!(confirm.targets().targets().len(), 1);
    assert_eq!(confirm.offer(), KillOffer::TerminateOrKill);
    assert_eq!(confirm.focused(), ConfirmButton::Terminate);
    assert_eq!(
        confirm.signal(ConfirmButton::Terminate),
        Some(Signal::Terminate)
    );
    for _ in 0..confirm.buttons().len() {
        confirm.focus_next();
    }
    assert_eq!(confirm.focused(), ConfirmButton::Terminate);
    confirm.focus_previous();
    assert_eq!(confirm.focused(), ConfirmButton::Cancel);
    assert_eq!(confirm.signal(ConfirmButton::Cancel), None);
    assert_eq!(confirm.signal(ConfirmButton::Send), None);

    assert_eq!(
        ConfirmButton::Kill.signal_for_offer(KillOffer::TerminateOrKill),
        Some(Signal::Kill)
    );
    assert_eq!(
        ConfirmButton::Send.signal_for_offer(KillOffer::TerminateOrKill),
        None
    );
}
