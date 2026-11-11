//! Per-connection Extended and Simple Query handler for the loopback PostgreSQL server.

use std::collections::BTreeMap;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use super::catalog::Catalog;
use super::frames::{
    read_cstring, read_i16, read_i32, write_command_complete, write_error, write_frame,
    write_ready, write_row_desc, write_text_rows, OID_TEXT,
};
use super::queries::{describe_query, parse_select_from};

pub(super) struct ConnState {
    pub pid: i32,
    pub read_only: bool,
    pub error_until_sync: bool,
    pub statements: BTreeMap<String, String>,
    pub portals: BTreeMap<String, (String, Vec<Vec<u8>>, i16)>,
    pub catalog: Arc<Mutex<Catalog>>,
}

impl ConnState {
    pub(super) fn handle_message(
        &mut self,
        tag: u8,
        body: &[u8],
        out: &mut impl Write,
    ) -> io::Result<()> {
        if self.error_until_sync {
            if tag == b'S' {
                self.error_until_sync = false;
                write_ready(out)?;
                out.flush()?;
            }
            return Ok(());
        }
        match tag {
            b'Q' => self.handle_simple_query(body, out),
            b'P' => self.handle_parse(body, out),
            b'D' => self.handle_describe(body, out),
            b'B' => self.handle_bind(body, out),
            b'E' => self.handle_execute(body, out),
            b'C' => write_frame(out, b'3', &[]),
            b'H' => out.flush(),
            b'S' => {
                write_ready(out)?;
                out.flush()
            }
            _ => Ok(()),
        }
    }

    fn handle_simple_query(&mut self, body: &[u8], out: &mut impl Write) -> io::Result<()> {
        let mut pos = 0;
        let query = read_cstring(body, &mut pos);
        if query.starts_with("SET SESSION CHARACTERISTICS AS TRANSACTION READ ONLY") {
            self.read_only = true;
            write_command_complete(out, "SET")?;
        } else if self.read_only && query.trim_start().starts_with("CREATE ") {
            write_error(
                out,
                "ERROR",
                "25006",
                "cannot execute in a read-only transaction",
            )?;
        } else if let Some(target) = parse_select_from(&query) {
            self.select_simple_rows(&target.0, &target.1, target.2, target.3, out)?;
        } else {
            self.catalog
                .lock()
                .expect("catalog lock")
                .apply_batch(&query);
            write_command_complete(out, "OK")?;
        }
        write_ready(out)?;
        out.flush()
    }

    fn select_simple_rows(
        &self,
        schema: &str,
        table: &str,
        limit: usize,
        offset: usize,
        out: &mut impl Write,
    ) -> io::Result<()> {
        let fetched = {
            let guard = self.catalog.lock().expect("catalog lock");
            guard
                .schemas
                .get(schema)
                .and_then(|s| s.get(table))
                .map(|rel| {
                    let names: Vec<String> = rel.columns.iter().map(|c| c.name.clone()).collect();
                    let rows: Vec<Vec<Option<String>>> =
                        rel.rows.iter().skip(offset).take(limit).cloned().collect();
                    (names, rows)
                })
        };
        let Some((names, rows)) = fetched else {
            return write_error(out, "ERROR", "42P01", "relation does not exist");
        };
        let cols: Vec<(&str, u32, i16)> = names.iter().map(|n| (n.as_str(), OID_TEXT, 0)).collect();
        write_row_desc(out, &cols)?;
        write_text_rows(out, &rows)?;
        write_command_complete(out, "SELECT")
    }

    fn handle_parse(&mut self, body: &[u8], out: &mut impl Write) -> io::Result<()> {
        let mut pos = 0;
        let name = read_cstring(body, &mut pos);
        let query = read_cstring(body, &mut pos);
        if let Some((schema, table, _, _)) = parse_select_from(&query) {
            let exists = self
                .catalog
                .lock()
                .expect("catalog lock")
                .schemas
                .get(&schema)
                .is_some_and(|s| s.contains_key(&table));
            if !exists {
                self.error_until_sync = true;
                return write_error(out, "ERROR", "42P01", "relation does not exist");
            }
        }
        self.statements.insert(name, query);
        write_frame(out, b'1', &[])
    }

    fn handle_describe(&self, body: &[u8], out: &mut impl Write) -> io::Result<()> {
        let kind = body[0];
        let mut pos = 1;
        let name = read_cstring(body, &mut pos);
        let (query, fmt) = if kind == b'S' {
            (self.statements.get(&name).expect("stmt").as_str(), 0)
        } else {
            let (q, _, f) = self.portals.get(&name).expect("portal");
            (q.as_str(), *f)
        };
        describe_query(&self.catalog, kind, query, fmt, out)
    }

    fn handle_bind(&mut self, body: &[u8], out: &mut impl Write) -> io::Result<()> {
        let mut pos = 0;
        let portal = read_cstring(body, &mut pos);
        let stmt = read_cstring(body, &mut pos);
        let num_fmt = usize::try_from(read_i16(body, &mut pos)).unwrap_or(0);
        pos += num_fmt * 2;
        let num_params = usize::try_from(read_i16(body, &mut pos)).unwrap_or(0);
        let mut params = Vec::with_capacity(num_params);
        for _ in 0..num_params {
            let len = usize::try_from(read_i32(body, &mut pos)).unwrap_or(0);
            params.push(body[pos..pos + len].to_vec());
            pos += len;
        }
        let num_res = read_i16(body, &mut pos);
        let res_fmt = if num_res > 0 {
            read_i16(body, &mut pos)
        } else {
            0
        };
        let query = self.statements.get(&stmt).cloned().unwrap_or_default();
        self.portals.insert(portal, (query, params, res_fmt));
        write_frame(out, b'2', &[])
    }

    fn handle_execute(&self, body: &[u8], out: &mut impl Write) -> io::Result<()> {
        let mut pos = 0;
        let portal = read_cstring(body, &mut pos);
        let (query, params, fmt) = self.portals.get(&portal).expect("portal exists");
        super::execute_portal(self.pid, &self.catalog, query, params, *fmt, out)?;
        write_command_complete(out, "SELECT 1")
    }
}
