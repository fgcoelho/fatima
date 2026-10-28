use std::collections::BTreeMap;

use serde_json::Value;

use crate::{FatimaError, Result};

pub fn interpolate_value(value: Value, env: &BTreeMap<String, String>) -> Result<Value> {
    match value {
        Value::String(value) => Ok(Value::String(interpolate_string(&value, env)?)),
        Value::Array(values) => values
            .into_iter()
            .map(|value| interpolate_value(value, env))
            .collect::<Result<Vec<_>>>()
            .map(Value::Array),
        Value::Object(values) => values
            .into_iter()
            .map(|(key, value)| Ok((key, interpolate_value(value, env)?)))
            .collect::<Result<serde_json::Map<_, _>>>()
            .map(Value::Object),
        value => Ok(value),
    }
}

pub fn interpolate_string(value: &str, env: &BTreeMap<String, String>) -> Result<String> {
    let mut output = String::new();
    let mut rest = value;

    while let Some(start) = rest.find("{env:") {
        output.push_str(&rest[..start]);
        let token_rest = &rest[start + 5..];
        let Some(end) = token_rest.find('}') else {
            output.push_str(&rest[start..]);
            return Ok(output);
        };
        let key = &token_rest[..end];
        let resolved = env.get(key).ok_or_else(|| {
            FatimaError::message(format!(
                "Missing environment variable referenced by interpolation: {key}"
            ))
        })?;
        output.push_str(resolved);
        rest = &token_rest[end + 1..];
    }

    output.push_str(rest);
    Ok(output)
}
