use serde_json::Value;

use super::builtins::{parse_model_value, validate_builtin};
use super::ModelSpec;
use crate::env::Secrets;
use crate::{FatimaError, ModelConfig, Result};

pub fn validate_environment(model: &ModelConfig, env: &Secrets) -> Result<()> {
    if model.is_empty() {
        return Err(FatimaError::message(
            "No model defined in fatima.json. Add `model` to use this command.",
        ));
    }

    let mut grouped = Vec::new();
    for (key, value) in model {
        let spec = super::model_spec(key, value);
        let mut messages = Vec::new();
        if let Err(message) = validate_builtin(spec.name, env.get(key)) {
            messages.push(message);
        }
        if let Some(message) = validate_allowed_values(&spec, env.get(key)) {
            messages.push(message);
        }
        if !messages.is_empty() {
            grouped.push((key.clone(), messages));
        }
    }

    if grouped.is_empty() {
        return Ok(());
    }

    let details = grouped
        .into_iter()
        .map(|(key, messages)| {
            format!(
                "{key}\n{}",
                messages
                    .into_iter()
                    .map(|m| format!("- {m}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    Err(FatimaError::message(format!(
        "Environment validation failed.\n\n{details}"
    )))
}

fn validate_allowed_values(spec: &ModelSpec<'_>, value: Option<&String>) -> Option<String> {
    let values = spec.args?.get("values")?.as_array()?;
    if values.contains(&parse_model_value(spec.name, value?)) {
        None
    } else {
        Some(format!(
            "Expected one of: {}",
            values
                .iter()
                .map(value_label)
                .collect::<Vec<_>>()
                .join(", ")
        ))
    }
}

fn value_label(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        _ => value.to_string(),
    }
}
