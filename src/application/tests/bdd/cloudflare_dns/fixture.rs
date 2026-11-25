//! Purpose: Stateful mock state and delayed gate for Cloudflare DNS BDD scenarios.

use std::sync::{Arc, Mutex};

use crate::application::cloudflare_jobs::CloudflareJobs;
use crate::application::tests::bdd::common::{await_gate, DelayGate};
use crate::models::cloudflare_dns::{
    Change, CloudflareError, CurrentHost, Proxy, Record, RecordId, Reply, Request, Ttl, Zone,
    ZoneId,
};

#[derive(Clone)]
pub struct SharedState {
    pub zones: Vec<Zone>,
    pub records_one: Vec<Record>,
    pub records_two: Vec<Record>,
    pub failure: Option<CloudflareError>,
    pub writes: Vec<Change>,
}

impl Default for SharedState {
    fn default() -> Self {
        Self {
            zones: vec![
                Zone {
                    id: ZoneId::new("z1"),
                    name: "example.com".into(),
                },
                Zone {
                    id: ZoneId::new("z2"),
                    name: "example.org".into(),
                },
            ],
            records_one: vec![Record {
                id: RecordId::new("r1"),
                kind: "A".into(),
                name: "www.example.com".into(),
                content: "192.0.2.1".into(),
                ttl: Ttl::Automatic,
                proxy: Proxy::Off,
            }],
            records_two: vec![Record {
                id: RecordId::new("r2"),
                kind: "A".into(),
                name: "api.example.org".into(),
                content: "203.0.113.9".into(),
                ttl: Ttl::Seconds(300),
                proxy: Proxy::On,
            }],
            failure: None,
            writes: Vec::new(),
        }
    }
}

pub struct Fixture {
    pub jobs: CloudflareJobs,
    pub state: Arc<Mutex<SharedState>>,
    pub gate: DelayGate,
}

impl Default for Fixture {
    fn default() -> Self {
        Self {
            jobs: CloudflareJobs::default(),
            state: Arc::new(Mutex::new(SharedState::default())),
            gate: DelayGate::default(),
        }
    }
}

impl Fixture {
    pub(super) fn failing(kind: &str) -> Self {
        let error = match kind {
            "missing token" => CloudflareError::MissingToken,
            "rate limit" => CloudflareError::RateLimited,
            "access denied" => CloudflareError::AccessDenied,
            other => panic!("unknown Cloudflare error {other}"),
        };
        Self {
            state: Arc::new(Mutex::new(SharedState {
                failure: Some(error),
                ..SharedState::default()
            })),
            ..Self::default()
        }
    }

    pub(super) fn delayed() -> Self {
        Self {
            gate: DelayGate::delayed(),
            ..Self::default()
        }
    }

    pub(super) fn fetch(&mut self) -> impl FnOnce(Request) -> Reply + Send + 'static {
        let state = Arc::clone(&self.state);
        let blocker = self.gate.blocker();
        move |req| {
            await_gate(blocker);
            let mut guard = state.lock().expect("state");
            respond(&mut guard, req)
        }
    }

    pub(super) fn write_count(&self) -> usize {
        self.state.lock().expect("state").writes.len()
    }
}

fn respond(state: &mut SharedState, req: Request) -> Reply {
    if let Some(err) = state.failure.clone() {
        return req.fail(err);
    }
    match req {
        Request::Zones => Reply::Zones(Ok(state.zones.clone())),
        Request::Records(zone) if zone.as_str() == "z1" => {
            Reply::Records(zone, Ok(state.records_one.clone()))
        }
        Request::Records(zone) => Reply::Records(zone, Ok(state.records_two.clone())),
        Request::Host => Reply::Host(Ok(CurrentHost {
            hostname: "host.example.com".into(),
            ipv4: Ok("198.51.100.42".into()),
        })),
        Request::Write(change) => apply_write(state, change),
    }
}

fn apply_write(state: &mut SharedState, change: Change) -> Reply {
    state.writes.push(change.clone());
    let record = match change {
        Change::Create(new) => Record {
            id: RecordId::new("r3"),
            kind: "A".into(),
            name: format!("{}.{}", new.name, new.zone.name),
            content: new.content,
            ttl: new.ttl,
            proxy: new.proxy,
        },
        Change::Update(upd) => Record {
            content: upd.content,
            ttl: upd.ttl,
            proxy: upd.proxy,
            ..upd.record
        },
    };
    state.records_one = vec![record.clone()];
    Reply::Written(Ok(record))
}
