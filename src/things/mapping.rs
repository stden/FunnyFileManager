//! Purpose: Machines as sessions: the checked values an SFTP session needs,
//! and the reason for each machine that cannot become one.

use std::collections::BTreeSet;

use crate::models::{Auth, HostName, Origin, Port, Protocol, Session, SessionName};
use crate::sessions::import::{Imported, SkipReason, Skipped};

use super::Machine;

/// The sessions the machines make, in the order given. A machine whose name or
/// host fails its check is skipped with that reason; a name an earlier machine
/// already took is skipped as a duplicate.
pub fn sessions_from<'a>(machines: impl IntoIterator<Item = &'a Machine>) -> Imported {
    let mut imported = Imported::default();
    let mut names = BTreeSet::new();
    for machine in machines {
        match session(machine) {
            Ok(session) if names.insert(session.name.clone()) => imported.sessions.push(session),
            Ok(session) => imported
                .skipped
                .push(Skipped::new(session.name.as_str(), SkipReason::Duplicate)),
            Err(reason) => imported
                .skipped
                .push(Skipped::new(&shown_name(machine), reason)),
        }
    }
    imported
}

/// One machine as an SFTP session on port 22, logging in with a password that
/// is asked for when it connects.
fn session(machine: &Machine) -> Result<Session, SkipReason> {
    Ok(Session {
        name: SessionName::new(&machine.name)?,
        protocol: Protocol::Sftp,
        host: HostName::new(machine.host())?,
        port: Port::SSH,
        user: None,
        auth: Auth::Password,
        remote_dir: None,
        origin: Origin::Things {
            machine: machine.id.clone(),
        },
    })
}

fn shown_name(machine: &Machine) -> String {
    if machine.name.trim().is_empty() {
        "(no name)".to_owned()
    } else {
        machine.name.clone()
    }
}
