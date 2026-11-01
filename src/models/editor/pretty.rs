//! Purpose: Pretty-print JSON and XML buffers in the built-in editor (`Ctrl+Alt+L`).

use std::path::Path;

use super::{Editor, EditorSelection};

mod xml;

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DocumentKind {
    Json,
    Xml,
}

impl Editor {
    /// Format the current buffer as pretty-printed JSON or XML (`Ctrl+Alt+L`).
    pub fn format_document(&mut self) {
        let source = self.lines.join("\n");
        let Some(kind) = detect_kind(&self.path, &source) else {
            self.notice = Some("Format is available for JSON and XML".to_owned());
            return;
        };
        let result = match kind {
            DocumentKind::Json => format_json(&source).map(|text| (text, "JSON")),
            DocumentKind::Xml => xml::format_xml(&source).map(|text| (text, "XML")),
        };
        match result {
            Ok((formatted, label)) => {
                self.apply_formatted(&formatted);
                self.notice = Some(format!("Formatted {label}"));
            }
            Err(error) => {
                self.notice = Some(error);
            }
        }
    }

    fn apply_formatted(&mut self, formatted: &str) {
        let new_lines: Vec<String> = formatted.split('\n').map(str::to_owned).collect();
        if new_lines != self.lines {
            let until = new_lines.len().max(1);
            self.lines = new_lines;
            self.cursor_line = self.cursor_line.min(self.lines.len().saturating_sub(1));
            self.clamp_col();
            self.selection = EditorSelection::None;
            self.note_dirty(0, until);
            self.mark_modified();
        }
    }
}

fn detect_kind(path: &Path, source: &str) -> Option<DocumentKind> {
    let ext = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
    if is_json_extension(ext) {
        return Some(DocumentKind::Json);
    }
    if is_xml_extension(ext) {
        return Some(DocumentKind::Xml);
    }
    let trimmed = source.trim_start();
    if trimmed.starts_with('{') || trimmed.starts_with('[') {
        return Some(DocumentKind::Json);
    }
    if trimmed.starts_with('<') {
        return Some(DocumentKind::Xml);
    }
    None
}

fn is_json_extension(ext: &str) -> bool {
    const JSON_EXTS: [&str; 5] = ["json", "jsonc", "geojson", "har", "webmanifest"];
    JSON_EXTS
        .iter()
        .any(|candidate| ext.eq_ignore_ascii_case(candidate))
}

fn is_xml_extension(ext: &str) -> bool {
    const XML_EXTS: [&str; 16] = [
        "xml", "xsd", "xsl", "xslt", "svg", "xhtml", "plist", "csproj", "vbproj", "fsproj",
        "props", "targets", "nuspec", "config", "pom", "netbox",
    ];
    XML_EXTS
        .iter()
        .any(|candidate| ext.eq_ignore_ascii_case(candidate))
}

fn format_json(source: &str) -> Result<String, String> {
    let value: serde_json::Value =
        serde_json::from_str(source).map_err(|err| format!("Invalid JSON: {err}"))?;
    serde_json::to_string_pretty(&value).map_err(|err| format!("Invalid JSON: {err}"))
}
