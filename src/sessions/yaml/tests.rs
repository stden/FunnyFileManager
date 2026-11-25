//! Purpose: Cover YAML profile read fallbacks, parse rejections, and encoder error mapping.

use super::{encode_yaml, load, parse_stored, StoreError};
use serde::{ser, Serialize, Serializer};
use std::fs;
use tempfile::TempDir;

struct FailingSerialize;

impl Serialize for FailingSerialize {
    fn serialize<S: Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
        Err(ser::Error::custom("injected YAML failure"))
    }
}

#[test]
fn yaml_load_marks_damaged_on_bad_profiles_or_corrupt_legacy_json() {
    let dir = TempDir::new().expect("temp dir");
    let yaml_path = dir.path().join("ffm.yml");
    fs::write(dir.path().join("sessions.json"), "{").expect("corrupt legacy");
    assert!(load(&yaml_path).is_damaged());

    fs::write(&yaml_path, "sessions: 123\n").expect("bad profiles type");
    assert!(load(&yaml_path).is_damaged());
    assert!(parse_stored(None).is_err());
}

#[test]
fn encode_yaml_maps_serialization_errors_to_write_errors() {
    let error = encode_yaml(&FailingSerialize).unwrap_err();
    assert!(matches!(error, StoreError::Write(_)));
    assert!(error.to_string().contains("injected YAML failure"));
}
