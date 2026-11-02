//! Purpose: A server whose command, argument or extension holds a control
//! character is refused, and the refusal names the field that holds it.

use super::*;

fn server(command: &str, args: &[&str], extensions: &[&str]) -> ServerSettings {
    ServerSettings {
        command: command.to_owned(),
        args: args.iter().map(|arg| (*arg).to_owned()).collect(),
        extensions: extensions.iter().map(|ext| (*ext).to_owned()).collect(),
        ..ServerSettings::default()
    }
}

#[test]
fn a_control_character_refuses_the_server_and_names_its_field() {
    let language = LanguageId::from_key("python");
    let command = server("python\n", &[], &[]);
    assert!(matches!(
        ServerForm::for_server(&language, &command),
        Err(OwnedField::Command)
    ));
    let argument = server("python", &["-x\t"], &[]);
    assert!(matches!(
        ServerForm::for_server(&language, &argument),
        Err(OwnedField::Args)
    ));
    let extension = server("python", &[], &["py\u{7}"]);
    assert!(matches!(
        ServerForm::for_server(&language, &extension),
        Err(OwnedField::Extensions)
    ));
}
