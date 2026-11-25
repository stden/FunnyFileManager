//! Purpose: Starter JSON arguments generator from MCP tool `inputSchema`.

use serde_json::{json, Map, Value};

/// Build a starter JSON object from a tool's `inputSchema` (`required` fields, else `{}`).
#[must_use]
pub fn default_args_template(schema: &Value) -> String {
    let mut object = Map::new();
    let required = schema.get("required").and_then(Value::as_array);
    let props = schema.get("properties").and_then(Value::as_object);
    if let (Some(req), Some(props)) = (required, props) {
        for key in req.iter().filter_map(Value::as_str) {
            let kind = props
                .get(key)
                .and_then(|p| p.get("type"))
                .and_then(Value::as_str);
            let sample = match kind {
                Some("integer" | "number") => json!(0),
                Some("boolean") => json!(false),
                Some("array") => json!([]),
                Some("object") => json!({}),
                _ => json!(""),
            };
            object.insert(key.to_owned(), sample);
        }
    }
    serde_json::to_string(&Value::Object(object)).unwrap_or_else(|_| "{}".to_owned())
}
