use serde::Serialize;

#[derive(Serialize)]
pub struct MarkdownEnvelope<T> {
    pub schema_version: &'static str,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub item: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<T>>,
}

pub fn ok_item<T>(item: T) -> MarkdownEnvelope<T> {
    MarkdownEnvelope {
        schema_version: "1",
        ok: true,
        code: None,
        message: None,
        item: Some(item),
        items: None,
    }
}

pub fn ok_items<T>(items: Vec<T>) -> MarkdownEnvelope<T> {
    MarkdownEnvelope {
        schema_version: "1",
        ok: true,
        code: None,
        message: None,
        item: None,
        items: Some(items),
    }
}

fn escape_inline(value: &str) -> String {
    value.replace('`', "\\`")
}

fn escape_table_cell(value: &str) -> String {
    escape_inline(value).replace('|', "\\|")
}

fn scalar_to_inline(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "_null_".to_owned(),
        serde_json::Value::Bool(v) => format!("`{v}`"),
        serde_json::Value::Number(v) => format!("`{v}`"),
        serde_json::Value::String(v) => {
            if v.contains('\n') {
                format!("\n```text\n{v}\n```")
            } else {
                format!("`{}`", escape_inline(v))
            }
        }
        _ => unreachable!("scalar_to_inline only supports scalar values"),
    }
}

fn is_scalar(value: &serde_json::Value) -> bool {
    matches!(value, serde_json::Value::Null | serde_json::Value::Bool(_) | serde_json::Value::Number(_) | serde_json::Value::String(_))
}

fn render_scalar_list(buf: &mut String, items: &[serde_json::Value], indent: usize) {
    let prefix = "  ".repeat(indent);
    for item in items {
        buf.push_str(&format!("{prefix}- {}\n", scalar_to_inline(item)));
    }
}

fn array_table_keys(items: &[serde_json::Value]) -> Option<Vec<String>> {
    let mut keys = None;
    for item in items {
        let obj = item.as_object()?;
        if obj.values().any(|value| !is_scalar(value)) {
            return None;
        }
        let current = obj.keys().cloned().collect::<Vec<_>>();
        if let Some(existing) = &keys {
            if *existing != current {
                return None;
            }
        } else {
            keys = Some(current);
        }
    }
    keys
}

fn should_render_table(items: &[serde_json::Value], keys: &[String]) -> bool {
    if items.len() > 100 {
        return false;
    }
    for item in items {
        let Some(obj) = item.as_object() else {
            return false;
        };
        for key in keys {
            let Some(value) = obj.get(key) else {
                return false;
            };
            if let serde_json::Value::String(text) = value {
                if text.contains('\n') || text.chars().count() > 80 {
                    return false;
                }
            }
        }
    }
    true
}

fn render_table(buf: &mut String, items: &[serde_json::Value], keys: &[String]) {
    buf.push('|');
    for key in keys {
        buf.push(' ');
        buf.push_str(key);
        buf.push_str(" |");
    }
    buf.push('\n');
    buf.push('|');
    for _ in keys {
        buf.push_str(" --- |");
    }
    buf.push('\n');
    for item in items {
        let obj = item.as_object().expect("table rows must be objects");
        buf.push('|');
        for key in keys {
            let value = obj.get(key).unwrap_or(&serde_json::Value::Null);
            let cell = match value {
                serde_json::Value::String(v) if v.contains('\n') => escape_table_cell(&v.replace('\n', "<br>")),
                serde_json::Value::String(v) => escape_table_cell(v),
                serde_json::Value::Null => String::new(),
                serde_json::Value::Bool(v) => v.to_string(),
                serde_json::Value::Number(v) => v.to_string(),
                _ => String::new(),
            };
            buf.push(' ');
            buf.push_str(&cell);
            buf.push_str(" |");
        }
        buf.push('\n');
    }
}

fn render_value(buf: &mut String, title: Option<&str>, value: &serde_json::Value, level: usize) {
    if let Some(title) = title {
        let heading = "#".repeat(level.max(1));
        buf.push_str(&format!("{heading} {title}\n\n"));
    }

    match value {
        serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_) => {
            buf.push_str(&format!("{}\n", scalar_to_inline(value)));
        }
        serde_json::Value::Array(items) => {
            if items.is_empty() {
                buf.push_str("_empty list_\n");
            } else if items.iter().all(is_scalar) {
                render_scalar_list(buf, items, 0);
            } else if let Some(keys) = array_table_keys(items).filter(|keys| should_render_table(items, keys)) {
                render_table(buf, items, &keys);
            } else {
                for (index, item) in items.iter().enumerate() {
                    render_value(buf, Some(&format!("item {}", index + 1)), item, level + 1);
                    if index + 1 != items.len() {
                        buf.push('\n');
                    }
                }
            }
        }
        serde_json::Value::Object(map) => {
            let mut scalar_keys = Vec::new();
            let mut nested = Vec::new();
            for (key, value) in map {
                if is_scalar(value) {
                    scalar_keys.push((key, value));
                } else {
                    nested.push((key, value));
                }
            }
            for (key, value) in &scalar_keys {
                buf.push_str(&format!("- **{key}**: {}\n", scalar_to_inline(value)));
            }
            if !scalar_keys.is_empty() && !nested.is_empty() {
                buf.push('\n');
            }
            for (index, (key, value)) in nested.iter().enumerate() {
                render_value(buf, Some(key), value, level + 1);
                if index + 1 != nested.len() {
                    buf.push('\n');
                }
            }
            if scalar_keys.is_empty() && nested.is_empty() {
                buf.push_str("_empty object_\n");
            }
        }
    }
}

fn render_document(value: &serde_json::Value) -> String {
    let mut buf = String::new();
    if let Some(obj) = value.as_object() {
        if obj.contains_key("schema_version") && obj.contains_key("ok") {
            buf.push_str("# Result\n\n");
            if let Some(schema) = obj.get("schema_version") {
                buf.push_str(&format!("- **schema_version**: {}\n", scalar_to_inline(schema)));
            }
            if let Some(ok) = obj.get("ok") {
                buf.push_str(&format!("- **ok**: {}\n", scalar_to_inline(ok)));
            }
            if let Some(code) = obj.get("code").filter(|value| !value.is_null()) {
                buf.push_str(&format!("- **code**: {}\n", scalar_to_inline(code)));
            }
            if let Some(message) = obj.get("message").filter(|value| !value.is_null()) {
                buf.push_str(&format!("- **message**: {}\n", scalar_to_inline(message)));
            }
            if let Some(item) = obj.get("item").filter(|value| !value.is_null()) {
                buf.push('\n');
                render_value(&mut buf, Some("item"), item, 2);
            }
            if let Some(items) = obj.get("items").filter(|value| !value.is_null()) {
                buf.push('\n');
                render_value(&mut buf, Some("items"), items, 2);
            }
            return buf;
        }
    }
    render_value(&mut buf, Some("Result"), value, 1);
    buf
}

pub async fn write_markdown<T>(value: &T) -> anyhow::Result<()>
where
    T: Serialize,
{
    use compio::buf::buf_try;
    use compio::fs;
    use compio::io::AsyncWriteExt;

    let value = serde_json::to_value(value)?;
    let mut out = render_document(&value).into_bytes();
    if !out.ends_with(b"\n") {
        out.push(b'\n');
    }
    buf_try!(@try fs::stdout().write_all(out).await);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize)]
    struct Demo {
        value: u32,
    }

    #[test]
    fn ok_item_uses_schema_v1() {
        let env = ok_item(Demo { value: 7 });
        let json = serde_json::to_value(env).unwrap();
        assert_eq!(json["schema_version"], "1");
        assert_eq!(json["ok"], true);
        assert_eq!(json["item"]["value"], 7);
        assert!(json.get("items").is_none());
    }

    #[test]
    fn ok_items_uses_items_field() {
        let env = ok_items(vec![Demo { value: 1 }, Demo { value: 2 }]);
        let json = serde_json::to_value(env).unwrap();
        assert_eq!(json["schema_version"], "1");
        assert_eq!(json["ok"], true);
        assert_eq!(json["items"].as_array().unwrap().len(), 2);
        assert!(json.get("item").is_none());
    }

    #[test]
    fn render_document_uses_markdown_table_for_item_lists() {
        let md = render_document(&serde_json::to_value(ok_items(vec![Demo { value: 1 }, Demo { value: 2 }])).unwrap());
        assert!(md.contains("# Result"));
        assert!(md.contains("## items"));
        assert!(md.contains("| value |"));
    }
}
