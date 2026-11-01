//! Purpose: Each scenario owns temporary files and a persistent, private MCP server.

use super::super::client::{Access, Client, SessionId};
use cucumber::given;
use serde_json::Value;

#[derive(cucumber::World, Default)]
pub(super) struct SessionWorld {
    case: Option<Case>,
}

pub(super) struct Case {
    pub client: Client,
    pub root: tempfile::TempDir,
    first: SessionId,
    second: Option<SessionId>,
    pub job: Option<JobId>,
    pub last: Value,
}

#[derive(Clone, Copy)]
pub(super) struct JobId(pub u64);

#[derive(Clone, Copy)]
pub(super) enum Role {
    First,
    Second,
}

impl Role {
    pub fn named(name: &str) -> Self {
        match name {
            "first" => Self::First,
            "second" => Self::Second,
            _ => panic!("unknown session role"),
        }
    }
}

impl std::fmt::Debug for SessionWorld {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SessionWorld")
            .field("initialized", &self.case.is_some())
            .finish()
    }
}

impl SessionWorld {
    pub fn case(&mut self) -> &mut Case {
        self.case.as_mut().expect("workspace Given step runs first")
    }

    pub async fn finish(&mut self) {
        if let Some(mut case) = self.case.take() {
            if let Some(second) = case.second.take() {
                case.client.close_session(&second).await;
            }
            case.client.close_session(&case.first).await;
            case.client.close().await;
        }
    }
}

impl Case {
    pub fn session(&mut self, role: Role) -> (&mut Client, SessionId) {
        let id = match role {
            Role::First => self.first.clone(),
            Role::Second => self
                .second
                .as_ref()
                .expect("second session Given step runs first")
                .clone(),
        };
        (&mut self.client, id)
    }
}

#[given("an isolated MCP workspace with an unchanged draft")]
async fn workspace(world: &mut SessionWorld) {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("draft.txt"), "alpha\n").unwrap();
    let mut client = Client::spawn(root.path(), Access::Application).await;
    let first = client.start(root.path()).await;
    world.case = Some(Case {
        client,
        root,
        first,
        second: None,
        job: None,
        last: Value::Null,
    });
}

#[given("a second independent background FFM session")]
async fn second_session(world: &mut SessionWorld) {
    let case = world.case();
    case.second = Some(case.client.start(case.root.path()).await);
}
