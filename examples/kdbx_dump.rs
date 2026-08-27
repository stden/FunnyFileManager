//! Dump every entry's title, username and password from a `.kdbx` you own.
//!
//! The master password is read from the `KDBX_PASSWORD` environment variable so
//! it never lands in argv or shell history. An optional key file path may be
//! given in `KDBX_KEYFILE`.
//!
//! ```bash
//! KDBX_PASSWORD='your master password' \
//!   cargo run --quiet --example kdbx_dump --features keepass -- KeyPass/COPS.kdbx
//! ```
//!
//! Output goes to stdout only — it is never written to a file or the app logs.

#[cfg(feature = "keepass")]
fn main() {
    use funny_file_manager::keepass;
    use std::path::Path;

    let Some(db_path) = std::env::args().nth(1) else {
        eprintln!("usage: kdbx_dump <file.kdbx>   (set KDBX_PASSWORD)");
        std::process::exit(2);
    };
    let Ok(password) = std::env::var("KDBX_PASSWORD") else {
        eprintln!("set the KDBX_PASSWORD environment variable to the master password");
        std::process::exit(2);
    };
    let keyfile = std::env::var("KDBX_KEYFILE").ok();

    let export = match keepass::export(
        Path::new(&db_path),
        &password,
        keyfile.as_deref().map(Path::new),
    ) {
        Ok(export) => export,
        Err(err) => {
            eprintln!("Failed to open {db_path}: {err}");
            std::process::exit(1);
        }
    };

    if let Some(name) = &export.database_name {
        println!("# database: {name}");
    }
    walk_group(&export.root, "");
}

#[cfg(feature = "keepass")]
fn walk_group(group: &funny_file_manager::keepass::ExportGroup, prefix: &str) {
    let here = if prefix.is_empty() {
        group.name.clone()
    } else {
        format!("{prefix}/{}", group.name)
    };
    for entry in &group.entries {
        let get = |k: &str| entry.fields.get(k).map_or("", String::as_str);
        println!(
            "{here}/{}\tuser: {}\tpass: {}\turl: {}",
            get("Title"),
            get("UserName"),
            get("Password"),
            get("URL"),
        );
    }
    for sub in &group.groups {
        walk_group(sub, &here);
    }
}

#[cfg(not(feature = "keepass"))]
fn main() {
    eprintln!("rebuild with --features keepass");
}
