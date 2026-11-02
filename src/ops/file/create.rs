//! Purpose: Shift+F4 and F7: create an empty file or a fresh `new_folder` in
//! the active panel, never overwriting what is already there.

use std::{fs, io};

use super::refresh_after_operation;
use crate::models::App;
use crate::status::Status;

pub fn handle_create_file(app: &mut App, filename: &str) -> io::Result<()> {
    let filename = filename.trim();
    if filename.is_empty() {
        app.set_status(Status::FilenameEmpty);
        return Ok(());
    }

    let file_path = app.get_active_panel().path.join(filename);
    // Exclusive create: no TOCTOU between exists-check and open (another
    // process cannot slip a file in and have us truncate it).
    match fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&file_path)
    {
        Ok(_) => {
            app.set_status(Status::CreatedFile(filename.to_owned()));
            refresh_after_operation(app);
            Ok(())
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            app.set_status(Status::FileExists(filename.to_owned()));
            Ok(())
        }
        Err(e) => Err(e),
    }
}

pub fn handle_mkdir(app: &mut App) -> io::Result<()> {
    let base_dir = app.get_active_panel().path.clone();
    // Exclusive `create_dir` + retry: no TOCTOU with another process claiming
    // `new_folder` between exists-check and create.
    for counter in 0..=9999 {
        let candidate = if counter == 0 {
            base_dir.join("new_folder")
        } else {
            base_dir.join(format!("new_folder_{counter}"))
        };
        match fs::create_dir(&candidate) {
            Ok(()) => {
                app.set_status(Status::CreatedDir(candidate.display().to_string()));
                refresh_after_operation(app);
                return Ok(());
            }
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "could not find unique folder name after 9999 attempts",
    ))
}
