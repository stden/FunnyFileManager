//! Purpose: File progress arithmetic and the scoped destination override.

use super::*;

fn destination(recorder: &Recorder) -> Option<PathBuf> {
    recorder
        .last()
        .transfer
        .map(|transfer| transfer.destination)
}

#[test]
fn advancing_progress_saturates_and_never_reopens_a_finished_file() {
    let file = FileProgress::Copying { done: 4, size: 20 };
    assert_eq!(
        file.advanced(3),
        FileProgress::Copying { done: 7, size: 20 }
    );
    assert_eq!(
        file.advanced(u64::MAX),
        FileProgress::Copying {
            done: u64::MAX,
            size: 20
        }
    );
    assert_eq!(FileProgress::Done.advanced(3), FileProgress::Done);
}

#[test]
fn destination_override_is_nested_and_restored_after_use() {
    let (recorder, _installed) = Recorder::install();
    let _run = begin(Operation::Move, &[PathBuf::from("source")]);
    let send = || sending(Path::new("source"), Path::new("staging"), 1);

    super::super::with_destination(Path::new("final"), || {
        send();
        assert_eq!(destination(&recorder), Some(PathBuf::from("final")));
        super::super::with_destination(Path::new("nested"), || {
            send();
            assert_eq!(destination(&recorder), Some(PathBuf::from("nested")));
        });
        send();
        assert_eq!(destination(&recorder), Some(PathBuf::from("final")));
    });
    send();
    assert_eq!(destination(&recorder), Some(PathBuf::from("staging")));
}

#[test]
fn destination_override_is_restored_during_unwinding() {
    let (recorder, _installed) = Recorder::install();
    let _run = begin(Operation::Move, &[PathBuf::from("source")]);
    let result = std::panic::catch_unwind(|| {
        super::super::with_destination(Path::new("final"), || panic!("stop copying"));
    });
    assert!(result.is_err());
    sending(Path::new("source"), Path::new("staging"), 1);
    assert_eq!(destination(&recorder), Some(PathBuf::from("staging")));
}
