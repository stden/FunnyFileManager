//! Purpose: Windows `ProcessHost` and `ProcessReader` backed by `src/platform`.

use std::io;
use std::time::Instant;

use crate::models::tasks::{NetProber, NetworkSample};
use crate::models::{
    ControlError, Pid, PriorityChange, ProcessHost, ProcessKey, ProcessReader, ProcessSample,
    Sample, SampleError, Signal, SystemPart, SystemSample,
};
use crate::platform::{
    list_processes, read_net_endpoints, read_network_adapters, read_system_sample,
    set_process_priority, terminate_process, WinOwnerCache,
};

use super::net_probe::SystemNetProber;

pub(super) struct WindowsHost {
    own: Pid,
}

impl WindowsHost {
    pub(super) fn system() -> Option<Self> {
        Some(Self {
            own: Pid::new(std::process::id())?,
        })
    }
}

impl ProcessHost for WindowsHost {
    fn reader(&self) -> Result<Box<dyn ProcessReader>, SampleError> {
        Ok(Box::new(WindowsReader::new()))
    }

    fn signal(&self, key: ProcessKey, signal: Signal) -> Result<(), ControlError> {
        terminate_process(key, signal, self.own)
    }

    fn set_priority(&self, key: ProcessKey, change: PriorityChange) -> Result<(), ControlError> {
        set_process_priority(key, change)
    }

    fn net_prober(&self) -> Option<Box<dyn NetProber>> {
        Some(Box::new(SystemNetProber::default()))
    }
}

struct WindowsReader {
    owners: WinOwnerCache,
    started: Instant,
}

type ProcListReader = fn(&mut WinOwnerCache) -> io::Result<(Vec<ProcessSample>, u32)>;

impl WindowsReader {
    fn new() -> Self {
        Self {
            owners: WinOwnerCache::default(),
            started: Instant::now(),
        }
    }

    fn sample_with(
        &mut self,
        read_sys: fn() -> io::Result<SystemSample>,
        read_procs: ProcListReader,
    ) -> Result<Sample, SampleError> {
        let system = read_sys().map_err(|err| SampleError::Unreadable {
            part: SystemPart::CpuTimes,
            kind: err.kind(),
        })?;
        let (processes, unreadable) =
            read_procs(&mut self.owners).map_err(|err| SampleError::Unreadable {
                part: SystemPart::ProcessList,
                kind: err.kind(),
            })?;
        let endpoints = read_net_endpoints();
        let adapters = read_network_adapters(&endpoints.ipv4_by_index);
        let network = NetworkSample {
            elapsed: Some(self.started.elapsed()),
            adapters,
            gateway: endpoints.gateway,
            dns_servers: endpoints.dns_servers,
        };
        Ok(Sample {
            system,
            processes,
            unreadable,
            network,
        })
    }
}

impl ProcessReader for WindowsReader {
    fn sample(&mut self) -> Result<Sample, SampleError> {
        self.sample_with(read_system_sample, list_processes)
    }
}

#[cfg(test)]
mod tests;
