//! Purpose: The PostgreSQL test server that `FFM_TEST_POSTGRES_URL` names, and a
//! schema of one test's own in it. Without that variable `server` finds none,
//! and a test that needs one returns at once.

use std::path::Path;

use postgres::config::Host;
use postgres::{Client, Config, NoTls};

use super::fixtures::name;
use crate::models::database::{ConnectionSpec, Driver, PostgresEndpoint};
use crate::test_support::EnvVarGuard;

const URL_VARIABLE: &str = "FFM_TEST_POSTGRES_URL";
const PASSWORD_VARIABLE: &str = "FFM_TEST_POSTGRES_PASSWORD";
const DEFAULT_PORT: u16 = 5432;

/// The server the URL names, split into what the driver is given.
pub(super) struct Server {
    url: String,
    pub(super) endpoint: PostgresEndpoint,
    pub(super) password: Option<String>,
}

/// The test server, or `None` when `FFM_TEST_POSTGRES_URL` is not set.
pub(super) fn server() -> Option<Server> {
    let url = std::env::var(URL_VARIABLE).ok()?;
    Some(from_url(url))
}

/// The server that `url` names. A URL that names a socket instead of a TCP host panics.
pub(super) fn from_url(url: String) -> Server {
    let config: Config = url
        .parse()
        .expect("the test server URL is a PostgreSQL URL");
    let Some(Host::Tcp(host)) = config.get_hosts().first() else {
        panic!("the test server URL names a TCP host");
    };
    let user = config.get_user().unwrap_or("postgres");
    let endpoint = PostgresEndpoint {
        host: host.clone(),
        port: config.get_ports().first().copied().unwrap_or(DEFAULT_PORT),
        database: config.get_dbname().unwrap_or(user).to_owned(),
        user: user.to_owned(),
        password_env: None,
    };
    let password = config
        .get_password()
        .map(|bytes| String::from_utf8(bytes.to_vec()).expect("a text password"));
    Server {
        url,
        endpoint,
        password,
    }
}

impl Server {
    /// A session as the explorer opens it: through the driver, read-only.
    pub(super) fn connect(&self) -> Client {
        super::postgres::connect(&self.endpoint, self.password.as_deref())
            .expect("connect to the test server")
    }

    /// A plain connection that may write, for the test to set its data up.
    pub(super) fn admin(&self) -> Client {
        Client::connect(&self.url, NoTls).expect("connect an admin to the test server")
    }

    /// The server as a configured connection. A password goes in the environment
    /// variable the entry names, which the guard keeps set while it lives.
    pub(super) fn spec(&self) -> (ConnectionSpec, Option<EnvVarGuard>) {
        let mut endpoint = self.endpoint.clone();
        let guard = self.password.as_deref().map(|password| {
            endpoint.password_env = Some(PASSWORD_VARIABLE.to_owned());
            EnvVarGuard::set(PASSWORD_VARIABLE, Path::new(password))
        });
        let spec = ConnectionSpec {
            name: name("server"),
            driver: Driver::Postgres(endpoint),
        };
        (spec, guard)
    }

    /// A driver session whose backend the server has terminated, so every
    /// statement on it fails.
    pub(super) fn dead_client(&self) -> Client {
        let mut victim = self.connect();
        let pid: i32 = victim
            .query_one("SELECT pg_backend_pid()", &[])
            .expect("the backend pid")
            .get(0);
        self.admin()
            .query_one("SELECT pg_terminate_backend($1)", &[&pid])
            .expect("terminate the backend");
        victim
    }
}

/// A schema made for one test in the server's database. It is dropped, with
/// everything in it, when the test ends, even by a panic.
pub(super) struct Scratch {
    admin: Client,
    pub(super) schema: String,
}

impl Scratch {
    pub(super) fn create(server: &Server, tag: &str) -> Self {
        let schema = format!("ffm_test_{tag}");
        let mut scratch = Self {
            admin: server.admin(),
            schema,
        };
        scratch.run("DROP SCHEMA IF EXISTS @schema CASCADE; CREATE SCHEMA @schema;");
        scratch
    }

    /// Run `sql` as the test's own writer; `@schema` stands for the scratch schema.
    pub(super) fn run(&mut self, sql: &str) {
        let statement = sql.replace("@schema", &self.schema);
        self.admin
            .batch_execute(&statement)
            .expect("the scratch statement runs");
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        self.run("DROP SCHEMA IF EXISTS @schema CASCADE");
    }
}
