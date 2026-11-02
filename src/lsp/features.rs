//! Purpose: Normalize standard LSP result shapes into editor-independent feature data.

use super::{positions::Range, types::Feature, uri};
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TextEdit {
    pub range: Range,
    pub new_text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Completion {
    pub label: String,
    pub kind: Option<u64>,
    pub detail: String,
    pub documentation: String,
    pub insert_text: String,
    pub edit: Option<TextEdit>,
    pub additional_edits: Vec<TextEdit>,
    pub snippet: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub range: Range,
    pub message: String,
    pub severity: Option<u32>,
}

impl Diagnostic {
    pub const fn label(&self) -> &'static str {
        match self.severity {
            Some(1) => "Error",
            Some(2) => "Warning",
            Some(4) => "Hint",
            _ => "Info",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Definition {
    pub path: PathBuf,
    pub range: Range,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reply {
    Completions(Vec<Completion>),
    Hover(String),
    Definitions(Vec<Definition>),
}

pub fn parse(feature: Feature, value: &Value) -> Result<Reply, &'static str> {
    match feature {
        Feature::Completion(_) => completions(value).map(Reply::Completions),
        Feature::Hover => hover(value).map(Reply::Hover),
        Feature::Definition => definitions(value).map(Reply::Definitions),
    }
}

fn hover(value: &Value) -> Result<String, &'static str> {
    if value.is_null() {
        return Ok(String::new());
    }
    let contents = value.get("contents").ok_or("Missing hover contents")?;
    if !valid_markup(contents) {
        return Err("Invalid hover contents");
    }
    Ok(markup(contents))
}

fn valid_markup(value: &Value) -> bool {
    value.is_string()
        || value
            .as_array()
            .is_some_and(|items| items.iter().all(valid_markup))
        || value.get("value").is_some_and(Value::is_string)
            && (value.get("language").is_some_and(Value::is_string)
                || matches!(value["kind"].as_str(), Some("plaintext" | "markdown")))
}

fn completions(value: &Value) -> Result<Vec<Completion>, &'static str> {
    if value.is_null() {
        return Ok(Vec::new());
    }
    let items = value
        .as_array()
        .or_else(|| value["items"].as_array())
        .ok_or("Invalid completion list")?;
    items.iter().take(200).map(completion).collect()
}

fn completion(item: &Value) -> Result<Completion, &'static str> {
    let label = item["label"]
        .as_str()
        .ok_or("Missing completion label")?
        .to_owned();
    let edit = item.get("textEdit").map(completion_edit).transpose()?;
    let additional_edits = item
        .get("additionalTextEdits")
        .map(|edits| serde_json::from_value(edits.clone()))
        .transpose()
        .map_err(crate::lsp::logged("Invalid additional completion edits"))?
        .unwrap_or_default();
    Ok(Completion {
        insert_text: item["insertText"].as_str().unwrap_or(&label).to_owned(),
        label,
        kind: item["kind"].as_u64(),
        detail: item["detail"].as_str().unwrap_or_default().to_owned(),
        documentation: markup(&item["documentation"]),
        edit,
        additional_edits,
        snippet: item["insertTextFormat"] == 2,
    })
}

fn completion_edit(edit: &Value) -> Result<TextEdit, &'static str> {
    let range = edit
        .get("range")
        .or_else(|| edit.get("replace"))
        .ok_or("Missing completion range")?;
    Ok(TextEdit {
        range: serde_json::from_value(range.clone())
            .map_err(crate::lsp::logged("Invalid completion range"))?,
        new_text: edit["newText"]
            .as_str()
            .ok_or("Missing completion text")?
            .to_owned(),
    })
}

pub fn markup(value: &Value) -> String {
    if let Some(text) = value.as_str() {
        return text.to_owned();
    }
    if let Some(values) = value.as_array() {
        return values.iter().map(markup).collect::<Vec<_>>().join("\n\n");
    }
    let text = value["value"].as_str().unwrap_or_default();
    value["language"].as_str().map_or_else(
        || text.to_owned(),
        |language| format!("```{language}\n{text}\n```"),
    )
}

fn definitions(value: &Value) -> Result<Vec<Definition>, &'static str> {
    if value.is_null() {
        return Ok(Vec::new());
    }
    value.as_array().map_or_else(
        || definition(value).map(|target| vec![target]),
        |items| items.iter().take(200).map(definition).collect(),
    )
}

fn definition(value: &Value) -> Result<Definition, &'static str> {
    let (uri, range) = if value.get("targetUri").is_some() {
        (&value["targetUri"], &value["targetSelectionRange"])
    } else {
        (&value["uri"], &value["range"])
    };
    Ok(Definition {
        path: uri
            .as_str()
            .and_then(uri::to_path)
            .ok_or("Definition is not a valid file URI")?,
        range: serde_json::from_value(range.clone())
            .map_err(crate::lsp::logged("Invalid definition range"))?,
    })
}
