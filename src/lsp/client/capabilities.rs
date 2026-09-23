//! Purpose: Advertise implemented features and honor negotiated sync and position encoding.

use crate::lsp::{
    document::{Document, SyncKind},
    rpc::Session,
    types::Feature,
};
use serde_json::{json, Value};

#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag is a separate server capability; every combination is valid"
)]
pub(super) struct Capabilities {
    pub open_close: bool,
    pub change: SyncKind,
    save: Option<bool>,
    completion: bool,
    hover: bool,
    definition: bool,
}

pub(super) fn initialize(root: &str) -> Value {
    json!({"processId":std::process::id(), "clientInfo":{"name":"FunnyFileManager","version":env!("CARGO_PKG_VERSION")},
    "rootUri":root, "workspaceFolders":[{"uri":root,"name":"workspace"}],
    "capabilities":{
        "general":{"positionEncodings":["utf-16"]},
        "workspace":{"configuration":true,"workspaceFolders":true},
        "textDocument":{
            "synchronization":{"didSave":true},
            "publishDiagnostics":{"versionSupport":true},
            "completion":{"completionItem":{"snippetSupport":false,"insertReplaceSupport":true,"documentationFormat":["markdown","plaintext"]}},
            "hover":{"contentFormat":["markdown","plaintext"]},
            "definition":{"linkSupport":true}
        }
    }})
}

impl Capabilities {
    pub fn parse(value: &Value) -> Result<Self, &'static str> {
        if !value.is_object() {
            return Err("Invalid LSP capabilities");
        }
        if value["positionEncoding"]
            .as_str()
            .is_some_and(|encoding| encoding != "utf-16")
        {
            return Err("Unsupported LSP position encoding");
        }
        let sync = &value["textDocumentSync"];
        let change = sync_kind(sync)?;
        Ok(Self {
            open_close: sync["openClose"]
                .as_bool()
                .unwrap_or_else(|| sync.is_number() && change != SyncKind::None),
            change,
            save: save_options(sync),
            completion: supported(&value["completionProvider"]),
            hover: supported(&value["hoverProvider"]),
            definition: supported(&value["definitionProvider"]),
        })
    }

    pub const fn supports(&self, feature: Feature) -> bool {
        match feature {
            Feature::Completion => self.completion,
            Feature::Hover => self.hover,
            Feature::Definition => self.definition,
        }
    }

    pub fn save(&self, doc: &Document, saved: bool, rpc: &mut Session) {
        if let (true, Some(include_text)) = (saved, self.save) {
            let mut params = json!({"textDocument":{"uri":doc.uri}});
            if include_text {
                params["text"] = json!(doc.snapshot.text);
            }
            rpc.notify("textDocument/didSave", params);
        }
    }
}

fn supported(value: &Value) -> bool {
    value == true || value.is_object()
}

fn sync_kind(sync: &Value) -> Result<SyncKind, &'static str> {
    match sync
        .as_u64()
        .or_else(|| sync["change"].as_u64())
        .unwrap_or(0)
    {
        0 => Ok(SyncKind::None),
        1 => Ok(SyncKind::Full),
        2 => Ok(SyncKind::Incremental),
        _ => Err("Invalid LSP synchronization kind"),
    }
}

fn save_options(sync: &Value) -> Option<bool> {
    match sync.get("save") {
        Some(Value::Bool(true)) => Some(false),
        Some(Value::Object(options)) => Some(
            options
                .get("includeText")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        ),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
