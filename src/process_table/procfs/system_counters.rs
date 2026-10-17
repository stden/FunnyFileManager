//! Purpose: The system-wide side of the `/proc` reader: `/proc/stat`,
//! `meminfo`, `loadavg` and `uptime` read into the counters and meters of a
//! sample. Only `/proc/stat` is required; the others cost only their meter.

use std::io;

use crate::models::{SampleError, SystemPart, SystemSample};

use super::files::{SystemFile, SYSTEM_CAP};
use super::parse;
use super::ProcFs;

impl ProcFs {
    /// The system-wide counters. Without `/proc/stat` there are no CPU
    /// shares, and the sample fails; the other files only lose their meter.
    pub(super) fn system_sample(&mut self) -> Result<SystemSample, SampleError> {
        let ticks = self.ticks;
        let times = self
            .read_system(SystemFile::Stat, |text| parse::cpu_times(text, ticks))
            .map_err(|error| SampleError::Unreadable {
                part: SystemPart::CpuTimes,
                kind: error.kind(),
            })?
            .ok_or(SampleError::Malformed(SystemPart::CpuTimes))?;
        let memory = self
            .read_system(SystemFile::MemInfo, parse::memory_info)
            .ok();
        Ok(SystemSample {
            total: times.total,
            cores: times.cores,
            memory: memory.and_then(|info| info.memory),
            swap: memory.and_then(|info| info.swap),
            load: self.optional(SystemFile::LoadAverage, parse::load_average),
            uptime: self.optional(SystemFile::Uptime, parse::uptime),
            running: times.running,
        })
    }

    /// A system-wide file, parsed.
    fn read_system<T>(&mut self, file: SystemFile, parse: impl FnOnce(&str) -> T) -> io::Result<T> {
        let path = self.root.join(file.name());
        self.buffer.read(&path, SYSTEM_CAP).map(|text| parse(&text))
    }

    /// An optional system-wide file's value: `None` when it cannot be read
    /// or says nothing.
    fn optional<T>(
        &mut self,
        file: SystemFile,
        parse: impl FnOnce(&str) -> Option<T>,
    ) -> Option<T> {
        self.read_system(file, parse).ok().flatten()
    }
}
