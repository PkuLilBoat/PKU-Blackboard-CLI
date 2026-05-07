use serde::Serialize;

#[derive(Serialize)]
pub struct JsonEnvelope<T> {
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

pub fn ok_item<T>(item: T) -> JsonEnvelope<T> {
    JsonEnvelope {
        schema_version: "1",
        ok: true,
        code: None,
        message: None,
        item: Some(item),
        items: None,
    }
}

pub fn ok_items<T>(items: Vec<T>) -> JsonEnvelope<T> {
    JsonEnvelope {
        schema_version: "1",
        ok: true,
        code: None,
        message: None,
        item: None,
        items: Some(items),
    }
}

pub async fn write_json<T>(value: &T) -> anyhow::Result<()>
where
    T: Serialize,
{
    use compio::buf::buf_try;
    use compio::fs;
    use compio::io::AsyncWriteExt;

    let mut out = serde_json::to_vec_pretty(value)?;
    out.push(b'\n');
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
}
