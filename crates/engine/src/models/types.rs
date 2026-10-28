use serde_json::Value;

pub struct ModelSpec<'a> {
    pub key: &'a str,
    pub name: &'a str,
    pub args: Option<&'a serde_json::Map<String, Value>>,
}

pub fn model_spec<'a>(key: &'a str, value: &'a Value) -> ModelSpec<'a> {
    match value {
        Value::String(name) => ModelSpec {
            key,
            name,
            args: None,
        },
        Value::Object(object) => ModelSpec {
            key,
            name: object
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("string"),
            args: object.get("args").and_then(Value::as_object),
        },
        _ => ModelSpec {
            key,
            name: "string",
            args: None,
        },
    }
}
