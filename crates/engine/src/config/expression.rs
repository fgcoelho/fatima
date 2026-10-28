use std::collections::BTreeMap;

use crate::{FatimaError, Result};

use super::interpolation::interpolate_string;

pub fn evaluate_environment_expression(
    expression: &str,
    env: &BTreeMap<String, String>,
) -> Result<String> {
    for part in expression.split("??") {
        let value = part.trim();
        if value.is_empty() {
            continue;
        }

        if let Some(literal) = quoted_literal(value) {
            if !literal.is_empty() {
                return Ok(literal);
            }
        } else if let Some(key) = env_token(value) {
            if let Some(value) = env.get(key) {
                if !value.is_empty() {
                    return Ok(value.clone());
                }
            }
        } else if !value.is_empty() {
            return interpolate_string(value, env);
        }
    }

    Err(FatimaError::message(
        "fatima.json `environment` must evaluate to a non-empty string.",
    ))
}

fn quoted_literal(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    if value.len() >= 2
        && ((bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'')
            || (bytes[0] == b'"' && bytes[value.len() - 1] == b'"'))
    {
        Some(value[1..value.len() - 1].to_string())
    } else {
        None
    }
}

fn env_token(value: &str) -> Option<&str> {
    value.strip_prefix("{env:")?.strip_suffix('}')
}
