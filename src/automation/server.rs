//! Purpose: Own the private automation transport independently of application effects.
use super::{
    dispatch,
    options::Options,
    protocol::{Request, VERSION},
    session::Session,
    transport,
};
use serde_json::json;
use std::io::{self, BufRead, Write};

pub(super) fn run(options: Options) -> io::Result<()> {
    serve(options, io::stdin().lock(), &mut io::stdout().lock())
}

pub(super) fn serve(
    options: Options,
    mut input: impl BufRead,
    output: &mut impl Write,
) -> io::Result<()> {
    let mut session = Session::start(options)?;
    transport::write(
        output,
        Ok(json!({"protocol_version": VERSION, "state": session.state()?})),
    )?;
    while next(&mut session, &mut input, output)? {}
    drop(session.close()?);
    Ok(())
}

fn next(
    session: &mut Session,
    input: &mut impl BufRead,
    output: &mut impl Write,
) -> io::Result<bool> {
    match transport::read(input) {
        Ok(Some(request)) => {
            let closing = matches!(&request, Request::Close {});
            transport::write(output, dispatch::request(session, request))?;
            Ok(!closing)
        }
        Ok(None) => Ok(false),
        Err(error) => {
            let fatal = error.kind() != io::ErrorKind::InvalidInput;
            transport::write(output, Err(error))?;
            Ok(!fatal)
        }
    }
}
