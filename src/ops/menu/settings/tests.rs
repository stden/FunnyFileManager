use super::*;
use crate::test_support::{EnvVarGuard, CONFIG_HOME};
use std::fs;

fn app_in(temp: &tempfile::TempDir) -> io::Result<App> {
    let panel = crate::models::Panel::new(temp.path().to_owned())?;
    Ok(App::from_panels(panel.clone(), panel))
}

fn write_file(path: &Path, text: &str) -> io::Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    fs::write(path, text)
}

mod applying;
mod loading_files;
mod saving;
