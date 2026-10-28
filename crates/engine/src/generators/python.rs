use super::spec::json;
use crate::env::Secrets;
use crate::models::model_spec;
use crate::FatimaConfig;

const PYTHON_TEMPLATE: &str = include_str!("../templates/private/env.py");

pub fn render_python(config: &FatimaConfig, loaded: &Secrets) -> String {
    PYTHON_TEMPLATE
        .replace(
            "    # __TYPE_BODY__\n    pass",
            &loaded
                .keys()
                .map(|key| {
                    format!(
                        "    {}: {}",
                        python_identifier(key),
                        python_type(config.model.get(key))
                    )
                })
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .replace(
            "    # __OBJECT_BODY__",
            &loaded
                .keys()
                .map(|key| format!("    {}: os.environ[{}],", json(key), json(key)))
                .collect::<Vec<_>>()
                .join("\n"),
        )
}

fn python_type(model: Option<&serde_json::Value>) -> &'static str {
    match model.map(|value| model_spec("", value).name) {
        Some("number" | "integer") => "float",
        Some("boolean") => "bool",
        Some("json") => "object",
        _ => "str",
    }
}

fn python_identifier(key: &str) -> String {
    let value: String = key
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect();
    if value
        .chars()
        .next()
        .map(|ch| ch.is_ascii_alphabetic() || ch == '_')
        .unwrap_or(false)
    {
        value
    } else {
        format!("_{value}")
    }
}
