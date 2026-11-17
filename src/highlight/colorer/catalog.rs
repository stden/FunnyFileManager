//! Purpose: Finding and sharing the one Colorer scheme database this process
//! loads — `COLORER_CATALOG`, then the usual install places.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use super::Colorer;

/// Where a catalog is looked for, in order, when `COLORER_CATALOG` is unset.
///
/// The first is where `FarColorer` keeps it on a Far install; the rest are where
/// a hand-built scheme checkout tends to land.
const CATALOG_CANDIDATES: [&str; 3] = [
    "/usr/share/colorer/catalog.xml",
    "/usr/local/share/colorer/catalog.xml",
    "/opt/colorer/catalog.xml",
];

/// The same, relative to the user's home — where `ffm.py colorer` and
/// `ffm.cmd colorer` install the schemes, since that needs no root.
const HOME_CANDIDATES: [&str; 2] = [
    ".local/share/ffm/colorer/catalog.xml",
    ".local/share/ffm/Colorer-schemes/base/catalog.base.xml",
];

/// The one scheme database this process loads.
static SHARED: OnceLock<Option<Colorer>> = OnceLock::new();

#[cfg(test)]
thread_local! {
    pub(super) static TEST_SHARED: std::cell::Cell<Option<&'static Colorer>> =
        const { std::cell::Cell::new(None) };
}

impl Colorer {
    /// Load the catalog named by `COLORER_CATALOG`, or the first of
    /// `CATALOG_CANDIDATES` that exists.
    ///
    /// `None` rather than an error: no scheme database is an ordinary state,
    /// not a failure, and the caller's answer to it is the same either way.
    /// The process's scheme database, loaded once.
    ///
    /// Shared rather than per-editor: `catalog.xml` pulls in hundreds of
    /// schemes, and re-reading them every time a file is opened would be felt.
    #[must_use]
    pub fn shared() -> Option<&'static Self> {
        #[cfg(test)]
        if let Some(override_shared) = TEST_SHARED.with(std::cell::Cell::get) {
            return Some(override_shared);
        }
        SHARED.get_or_init(Self::discover).as_ref()
    }

    #[must_use]
    pub(super) fn discover() -> Option<Self> {
        Self::discover_in(
            std::env::var_os("COLORER_CATALOG").map(PathBuf::from),
            &CATALOG_CANDIDATES,
            crate::config::user_home(),
        )
    }

    #[must_use]
    pub(super) fn discover_in(
        env_catalog: Option<PathBuf>,
        candidates: &[&str],
        home: Option<PathBuf>,
    ) -> Option<Self> {
        let catalog = env_catalog
            .filter(|path| path.is_file())
            .or_else(|| {
                candidates
                    .iter()
                    .map(PathBuf::from)
                    .find(|path| path.is_file())
            })
            .or_else(|| {
                // The installer scripts put the schemes under the same tail
                // of the home on every platform.
                let home = home?;
                HOME_CANDIDATES
                    .iter()
                    .map(|tail| home.join(tail))
                    .find(|path| path.is_file())
            })?;
        Self::open(&catalog)
    }

    /// The catalog this was loaded from, for reporting.
    #[must_use]
    pub fn catalog(&self) -> &Path {
        &self.catalog
    }
}
