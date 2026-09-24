//! Purpose: Tests for the raw sample types: state letters, and a reader that
//! crosses to the sampling thread.

use std::time::Duration;

use super::{CpuCounters, ProcessReader, ProcessState, Sample, SampleError, SystemSample};

#[test]
fn each_kernel_state_letter_names_its_state_and_reads_back() {
    let cases = [
        (b'R', ProcessState::Running),
        (b'S', ProcessState::Sleeping),
        (b'D', ProcessState::DiskSleep),
        (b'Z', ProcessState::Zombie),
        (b'T', ProcessState::Stopped),
        (b't', ProcessState::TracingStop),
        (b'X', ProcessState::Dead),
        (b'I', ProcessState::Idle),
        (b'P', ProcessState::Parked),
    ];
    for (letter, state) in cases {
        assert_eq!(ProcessState::from_letter(letter), state);
        assert_eq!(state.letter(), char::from(letter));
    }
}

#[test]
fn an_unnamed_letter_is_kept_only_when_it_is_visible_ascii() {
    let cases = [
        (b'W', 'W'),
        (b'x', 'x'),
        (b'K', 'K'),
        (b'~', '~'),
        (b' ', '?'),
        (0x1b, '?'),
        (0x00, '?'),
        (0x7f, '?'),
        (0xff, '?'),
    ];
    for (byte, shown) in cases {
        let state = ProcessState::from_letter(byte);
        assert!(matches!(state, ProcessState::Other(_)), "{byte:#04x}");
        assert_eq!(state.letter(), shown, "{byte:#04x}");
    }
}

/// Counts its samples; each reports one more second of idle time.
struct CountingReader {
    taken: u32,
}

impl ProcessReader for CountingReader {
    fn sample(&mut self) -> Result<Sample, SampleError> {
        self.taken = self
            .taken
            .checked_add(1)
            .ok_or(SampleError::WorkerStopped)?;
        let idle = Duration::from_secs(u64::from(self.taken));
        Ok(Sample {
            system: SystemSample {
                total: CpuCounters {
                    idle,
                    ..CpuCounters::default()
                },
                cores: Vec::new(),
                memory: None,
                swap: None,
                load: None,
                uptime: None,
                running: None,
            },
            processes: Vec::new(),
            unreadable: 0,
        })
    }
}

#[test]
fn a_reader_moves_to_the_sampling_thread() {
    let mut reader: Box<dyn ProcessReader> = Box::new(CountingReader { taken: 0 });
    let idle = std::thread::spawn(move || {
        [reader.sample(), reader.sample()]
            .map(|result| result.map(|sample| sample.system.total.idle))
    })
    .join()
    .expect("the sampling thread finished");
    assert_eq!(
        idle,
        [Ok(Duration::from_secs(1)), Ok(Duration::from_secs(2))]
    );
}
