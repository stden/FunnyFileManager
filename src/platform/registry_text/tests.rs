//! Purpose: Pin how a registry string value's UTF-16 becomes text and back.

use super::*;

fn units(text: &str) -> Vec<u16> {
    text.encode_utf16().collect()
}

#[test]
fn every_terminating_nul_is_dropped_and_none_is_needed() {
    let path = r"%SystemRoot%\system32;C:\Tools";
    for nuls in 0..3 {
        let mut data = units(path);
        data.extend(std::iter::repeat_n(0, nuls));
        assert_eq!(decode(&data).expect("text"), path, "{nuls} NULs");
    }
    assert_eq!(decode(&[]).expect("empty"), "");
    assert_eq!(decode(&[0]).expect("only a NUL"), "");
}

#[test]
fn text_is_written_with_one_terminating_nul_and_reads_back() {
    let path = r"C:\Program Files\Tools;%USERPROFILE%\bin;C:\Données";
    let data = encode(path).expect("data");
    assert_eq!(data.last(), Some(&0));
    assert_eq!(data.iter().filter(|&&unit| unit == 0).count(), 1);
    assert_eq!(decode(&data).expect("text"), path);
    assert_eq!(encode("").expect("empty"), [0]);
}

/// A NUL inside the data, or a lone surrogate, is text the editor could not
/// write back as it was read: refused rather than altered.
#[test]
fn data_that_is_not_plain_text_is_refused() {
    let mut inner_nul = units("C:\\a");
    inner_nul.push(0);
    inner_nul.extend(units("C:\\b"));
    let lone_surrogate = [u16::from(b'C'), 0xD800, u16::from(b'x')];
    for data in [inner_nul.as_slice(), &lone_surrogate] {
        let error = decode(data).expect_err("not text");
        assert_eq!(error.kind(), io::ErrorKind::InvalidData, "{data:?}");
    }
    let error = encode("C:\\a\0C:\\b").expect_err("NUL");
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
}

#[test]
fn each_kind_is_named_as_the_registry_names_it() {
    assert_eq!(RegistryStringKind::Plain.label(), "REG_SZ");
    assert_eq!(RegistryStringKind::Expandable.label(), "REG_EXPAND_SZ");
}
