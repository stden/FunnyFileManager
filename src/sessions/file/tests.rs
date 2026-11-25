//! Purpose: Verify JSON serialization error mapping on the sessions file store.

use super::{encode_json, StoreError};
use serde::{ser, Serialize, Serializer};

struct FailingSerialize;

impl Serialize for FailingSerialize {
    fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(ser::Error::custom("injected JSON failure"))
    }
}

#[test]
fn encode_json_maps_serialization_errors_to_write_errors() {
    let error = encode_json(&FailingSerialize).unwrap_err();
    assert!(matches!(error, StoreError::Write(_)));
    assert!(error.to_string().contains("injected JSON failure"));
}
