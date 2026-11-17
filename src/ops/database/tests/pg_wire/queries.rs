//! Catalog query evaluation and SELECT statement parsing for the loopback PostgreSQL server.

use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use super::catalog::{unquote, Catalog, Relation};
use super::frames::{write_param_desc, write_row_desc, OID_BOOL, OID_INT4, OID_TEXT};
use crate::ops::database::sql;

pub(super) fn parse_select_from(query: &str) -> Option<(String, String, usize, usize)> {
    let (before_offset, offset_str) = query.rsplit_once(" OFFSET ")?;
    let (before_limit, limit_str) = before_offset.rsplit_once(" LIMIT ")?;
    let (_, from_target) = before_limit.split_once(" FROM ")?;
    let offset = offset_str.trim().parse().ok()?;
    let limit = limit_str.trim().parse().ok()?;
    let (raw_schema, raw_table) = from_target.trim().split_once("\".\"")?;
    let schema = unquote(&format!("{raw_schema}\""));
    let table = unquote(&format!("\"{raw_table}"));
    Some((schema, table, limit, offset))
}

pub(super) fn query_catalog(
    guard: &Catalog,
    query: &str,
    params: &[Vec<u8>],
) -> Vec<Vec<Option<String>>> {
    if query == sql::POSTGRES_SCHEMAS {
        return guard
            .schemas
            .keys()
            .map(|n| vec![Some(n.clone())])
            .collect();
    }
    let p0 = param_str(params, 0);
    if query == sql::POSTGRES_TABLES {
        return guard
            .schemas
            .get(&p0)
            .into_iter()
            .flat_map(|t| {
                t.iter()
                    .map(|(n, r)| vec![Some(n.clone()), Some(r.kind.to_owned())])
            })
            .collect();
    }
    let p1 = param_str(params, 1);
    guard
        .schemas
        .get(&p0)
        .and_then(|s| s.get(&p1))
        .map_or_else(Vec::new, |rel| relation_rows(rel, query))
}

fn param_str(params: &[Vec<u8>], index: usize) -> String {
    params
        .get(index)
        .map(|b| String::from_utf8_lossy(b).into_owned())
        .unwrap_or_default()
}

fn relation_rows(rel: &Relation, query: &str) -> Vec<Vec<Option<String>>> {
    if query == sql::POSTGRES_PRIMARY_KEYS {
        return rel
            .columns
            .iter()
            .filter(|c| c.is_primary)
            .map(|c| vec![Some(c.name.clone())])
            .collect();
    }
    rel.columns
        .iter()
        .map(|c| {
            vec![
                Some(c.name.clone()),
                Some(c.data_type.clone()),
                Some(c.is_nullable.to_owned()),
            ]
        })
        .collect()
}

pub(super) fn describe_query(
    catalog: &Arc<Mutex<Catalog>>,
    kind: u8,
    query: &str,
    fmt: i16,
    out: &mut impl Write,
) -> io::Result<()> {
    if kind == b'S' {
        let params: &[u32] = if query.contains("pg_terminate_backend") {
            &[OID_INT4]
        } else if query == sql::POSTGRES_TABLES {
            &[OID_TEXT]
        } else if query == sql::POSTGRES_PRIMARY_KEYS || query == sql::POSTGRES_COLUMNS {
            &[OID_TEXT, OID_TEXT]
        } else {
            &[]
        };
        write_param_desc(out, params)?;
    }
    if let Some((schema, table, _, _)) = parse_select_from(query) {
        let names: Vec<String> = catalog.lock().expect("catalog lock").schemas[&schema][&table]
            .columns
            .iter()
            .map(|c| c.name.clone())
            .collect();
        let cols: Vec<(&str, u32, i16)> =
            names.iter().map(|n| (n.as_str(), OID_TEXT, fmt)).collect();
        return write_row_desc(out, &cols);
    }
    describe_static_rows(query, fmt, out)
}

fn describe_static_rows(query: &str, fmt: i16, out: &mut impl Write) -> io::Result<()> {
    let single = [("value", OID_TEXT, fmt)];
    let pid_col = [("pg_backend_pid", OID_INT4, fmt)];
    let term_col = [("pg_terminate_backend", OID_BOOL, fmt)];
    let tbl_cols = [("table_name", OID_TEXT, fmt), ("table_type", OID_TEXT, fmt)];
    let col_cols = [
        ("column_name", OID_TEXT, fmt),
        ("data_type", OID_TEXT, fmt),
        ("is_nullable", OID_TEXT, fmt),
    ];
    let spec: &[(&str, u32, i16)] = if query.contains("pg_backend_pid") {
        &pid_col
    } else if query.contains("pg_terminate_backend") {
        &term_col
    } else if query == sql::POSTGRES_TABLES {
        &tbl_cols
    } else if query == sql::POSTGRES_COLUMNS {
        &col_cols
    } else {
        &single
    };
    write_row_desc(out, spec)
}
