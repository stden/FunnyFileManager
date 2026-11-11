//! In-memory PostgreSQL catalog and DDL/DML interpreter for the loopback wire server.

use std::collections::BTreeMap;
use std::net::{Shutdown, TcpStream};

pub(super) struct ColumnDef {
    pub name: String,
    pub data_type: String,
    pub is_nullable: &'static str,
    pub is_primary: bool,
}

pub(super) struct Relation {
    pub kind: &'static str,
    pub columns: Vec<ColumnDef>,
    pub rows: Vec<Vec<Option<String>>>,
}

pub(super) struct Catalog {
    pub schemas: BTreeMap<String, BTreeMap<String, Relation>>,
    pub backends: BTreeMap<i32, TcpStream>,
    pub next_pid: i32,
}

impl Default for Catalog {
    fn default() -> Self {
        let mut schemas = BTreeMap::new();
        schemas.insert("public".to_owned(), BTreeMap::new());
        Self {
            schemas,
            backends: BTreeMap::new(),
            next_pid: 1000,
        }
    }
}

impl Catalog {
    pub(super) fn register_backend(&mut self, stream: &TcpStream) -> i32 {
        self.next_pid += 1;
        if let Ok(cloned) = stream.try_clone() {
            self.backends.insert(self.next_pid, cloned);
        }
        self.next_pid
    }

    pub(super) fn terminate_backend(&mut self, pid: i32) {
        if let Some(stream) = self.backends.remove(&pid) {
            drop(stream.shutdown(Shutdown::Both));
        }
    }

    pub(super) fn apply_batch(&mut self, batch: &str) {
        for stmt in batch.split(';').map(str::trim).filter(|s| !s.is_empty()) {
            self.apply_statement(stmt);
        }
    }

    fn apply_statement(&mut self, stmt: &str) {
        if let Some(rest) = stmt.strip_prefix("DROP SCHEMA IF EXISTS ") {
            self.schemas
                .remove(rest.trim_end_matches(" CASCADE").trim());
        } else if let Some(rest) = stmt.strip_prefix("CREATE SCHEMA ") {
            self.schemas.entry(rest.trim().to_owned()).or_default();
        } else if let Some(rest) = stmt.strip_prefix("CREATE TABLE ") {
            self.create_table(rest);
        } else if let Some(rest) = stmt.strip_prefix("CREATE VIEW ") {
            self.create_view(rest);
        } else if let Some(rest) = stmt.strip_prefix("INSERT INTO ") {
            self.insert_into(rest);
        }
    }

    fn create_table(&mut self, rest: &str) {
        let (schema, after_schema) = rest.split_once('.').expect("schema.table");
        let (raw_table, after_table) = after_schema.split_once('(').expect("table (cols)");
        let inside = after_table.trim().strip_suffix(')').expect("closing paren");
        let (cols_part, composite_pks) = split_composite_pk(inside);
        let columns = cols_part
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|spec| parse_column_def(spec, &composite_pks))
            .collect();
        let rel = Relation {
            kind: "BASE TABLE",
            columns,
            rows: Vec::new(),
        };
        self.schemas
            .entry(schema.trim().to_owned())
            .or_default()
            .insert(unquote(raw_table.trim()), rel);
    }

    fn create_view(&mut self, rest: &str) {
        let (schema, after_schema) = rest.split_once('.').expect("schema.view");
        let (raw_view, select_part) = after_schema.split_once(" AS ").expect("view AS");
        let col_name = if select_part.contains("AS shown") {
            "shown"
        } else {
            "id"
        };
        let rel = Relation {
            kind: "VIEW",
            columns: vec![ColumnDef {
                name: col_name.to_owned(),
                data_type: "text".to_owned(),
                is_nullable: "YES",
                is_primary: false,
            }],
            rows: Vec::new(),
        };
        self.schemas
            .entry(schema.trim().to_owned())
            .or_default()
            .insert(unquote(raw_view.trim()), rel);
    }

    fn insert_into(&mut self, rest: &str) {
        let (schema, after_schema) = rest.split_once('.').expect("schema.table");
        let schema_map = self.schemas.get_mut(schema.trim()).expect("schema exists");
        if after_schema.contains("generate_series(1, 450)") {
            schema_map
                .get_mut("big")
                .expect("big")
                .rows
                .extend((1..=450).map(|n| vec![Some(n.to_string())]));
        } else if after_schema.starts_with("kinds ") {
            let rel = schema_map.get_mut("kinds").expect("kinds");
            rel.rows.push(
                [
                    "1",
                    "true",
                    "2024-01-02 03:04:05",
                    "\\xdeadbeef",
                    "tab\there\nbreak",
                ]
                .map(|s| Some(s.to_owned()))
                .to_vec(),
            );
            rel.rows
                .push(vec![Some("2".to_owned()), None, None, None, None]);
        } else if after_schema.starts_with("items ") {
            let rel = schema_map.get_mut("items").expect("items");
            rel.rows
                .push(vec![Some("1".to_owned()), Some("apple".to_owned()), None]);
            rel.rows
                .push(["2", "pear", "ripe"].map(|s| Some(s.to_owned())).to_vec());
        }
    }
}

pub(super) fn unquote(ident: &str) -> String {
    ident
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .map_or_else(|| ident.to_owned(), |s| s.replace("\"\"", "\""))
}

fn split_composite_pk(inside: &str) -> (&str, Vec<String>) {
    let Some((cols, pk_part)) = inside.split_once("PRIMARY KEY (") else {
        return (inside, Vec::new());
    };
    let pk_list = pk_part
        .trim_end_matches(')')
        .split(',')
        .map(|s| s.trim().to_owned())
        .collect();
    (cols.trim_end().trim_end_matches(','), pk_list)
}

fn parse_column_def(spec: &str, composite_pks: &[String]) -> ColumnDef {
    let (name, tail) = parse_col_name(spec);
    let data_type = tail.split_whitespace().next().unwrap_or("text").to_owned();
    let upper = tail.to_ascii_uppercase();
    let is_primary = upper.contains("PRIMARY KEY") || composite_pks.iter().any(|pk| pk == &name);
    let is_nullable = if is_primary || upper.contains("NOT NULL") {
        "NO"
    } else {
        "YES"
    };
    ColumnDef {
        name,
        data_type,
        is_nullable,
        is_primary,
    }
}

fn parse_col_name(spec: &str) -> (String, &str) {
    if spec.starts_with('"') {
        if let Some((quoted, tail)) = spec.rsplit_once("\" ") {
            return (unquote(&format!("{quoted}\"")), tail.trim());
        }
    }
    spec.split_once(' ')
        .map_or_else(|| (spec.to_owned(), ""), |(n, t)| (n.to_owned(), t.trim()))
}
