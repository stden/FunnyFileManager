//! Purpose: Tests for `:ftp` and `:netdrives` command parsing and usage errors.

use super::*;

#[test]
fn ftp_and_netdrives_commands_parse_and_validate_arguments() {
    use super::NetDrivesCommand;

    assert_eq!(Command::parse("ftp"), Command::Ftp(None));
    assert_eq!(
        Command::parse("ftp ftps://example.test:990/pub"),
        Command::Ftp(Some("ftps://example.test:990/pub".to_owned()))
    );
    assert!(matches!(
        Command::parse("ftp one two"),
        Command::Invalid { .. }
    ));

    assert_eq!(
        Command::parse("netdrives"),
        Command::NetDrives(NetDrivesCommand::Connect)
    );
    assert_eq!(
        Command::parse("netdrives connect"),
        Command::NetDrives(NetDrivesCommand::Connect)
    );
    assert_eq!(
        Command::parse("netdrives edit"),
        Command::NetDrives(NetDrivesCommand::Edit)
    );
    assert_eq!(
        Command::parse("netdrives list"),
        Command::NetDrives(NetDrivesCommand::List)
    );
    assert!(matches!(
        Command::parse("netdrives bad"),
        Command::Invalid { .. }
    ));
}
