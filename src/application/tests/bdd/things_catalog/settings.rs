//! Purpose: The catalog's two environment variables, set or unset for the one menu
//! action that reads them, under the crate's environment lock and put back after.

use std::{env, ffi::OsString, sync::MutexGuard};

use crate::{
    test_support::env_lock,
    things::{TOKEN_VARIABLE, URL_VARIABLE},
};

const CONFIGURED_URL: &str = "https://things.example.test";
const CONFIGURED_TOKEN: &str = "fixture-token";

/// Runs `read` with the catalog's two variables set to the test's settings, or unset.
pub(super) fn with_catalog_settings<T>(configured: bool, read: impl FnOnce() -> T) -> T {
    let _settings = CatalogSettings::apply(configured);
    read()
}

/// Holds the environment lock, and the catalog variables' previous values, until
/// dropped; the previous values are put back then.
struct CatalogSettings {
    _lock: MutexGuard<'static, ()>,
    previous: [(&'static str, Option<OsString>); 2],
}

impl CatalogSettings {
    fn apply(configured: bool) -> Self {
        let lock = env_lock();
        let settings = [
            (URL_VARIABLE, CONFIGURED_URL),
            (TOKEN_VARIABLE, CONFIGURED_TOKEN),
        ];
        let previous = settings.map(|(name, value)| {
            let before = env::var_os(name);
            if configured {
                env::set_var(name, value);
            } else {
                env::remove_var(name);
            }
            (name, before)
        });
        Self {
            _lock: lock,
            previous,
        }
    }
}

impl Drop for CatalogSettings {
    fn drop(&mut self) {
        for (name, before) in &self.previous {
            match before {
                Some(value) => env::set_var(name, value),
                None => env::remove_var(name),
            }
        }
    }
}
