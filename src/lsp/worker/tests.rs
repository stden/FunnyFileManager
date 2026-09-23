//! Purpose: Preserve ordered protocol messages under bounded writer backpressure.

use super::*;
use serde_json::json;
use std::sync::mpsc;

#[test]
fn a_full_writer_retries_messages_in_order_without_losing_them() {
    let (sender, receiver) = mpsc::sync_channel(1);
    let mut messages = vec![json!(1), json!(2), json!(3)];
    flush_messages(&mut messages, |message| sender.try_send(message)).unwrap();
    assert_eq!(receiver.try_recv().unwrap(), json!(1));
    assert_eq!(messages, [json!(2), json!(3)]);
    flush_messages(&mut messages, |message| sender.try_send(message)).unwrap();
    assert_eq!(receiver.try_recv().unwrap(), json!(2));
    flush_messages(&mut messages, |message| sender.try_send(message)).unwrap();
    assert_eq!(receiver.try_recv().unwrap(), json!(3));
    assert!(messages.is_empty());
    drop(receiver);
    messages.push(json!(4));
    assert!(flush_messages(&mut messages, |message| sender.try_send(message)).is_err());
}

#[test]
fn an_unresponsive_writer_cannot_grow_the_outbox_without_bound() {
    let mut messages = vec![json!(0); 1025];
    assert!(flush_messages(&mut messages, |_| panic!("must reject before sending")).is_err());
}
