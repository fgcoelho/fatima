use serde_json::Value;

pub fn validate_builtin(name: &str, value: Option<&String>) -> std::result::Result<(), String> {
    let missing = missing_message(name)?;
    let value = value
        .map(String::as_str)
        .ok_or_else(|| missing.to_string())?;

    match name {
        "string" => Ok(()),
        "nonempty" if !value.trim().is_empty() => Ok(()),
        "email" if is_email(value) => Ok(()),
        "url" if is_url(value) => Ok(()),
        "uuid" if is_uuid(value) => Ok(()),
        "number" if !value.trim().is_empty() && value.parse::<f64>().is_ok() => Ok(()),
        "integer"
            if value
                .parse::<f64>()
                .map(|n| n.fract() == 0.0)
                .unwrap_or(false) =>
        {
            Ok(())
        }
        "boolean" if parse_boolean(value).is_some() => Ok(()),
        "json" if serde_json::from_str::<Value>(value).is_ok() => Ok(()),
        "pem" if value.contains("-----BEGIN ") && value.contains("-----END ") => Ok(()),
        "sk" if value.starts_with("sk-") || value.starts_with("sk_") => Ok(()),
        "bearer" if value.starts_with("Bearer ") && value.len() > 7 => Ok(()),
        _ => Err(missing.to_string()),
    }
}

pub fn parse_model_value(name: &str, value: &str) -> Value {
    match name {
        "number" | "integer" => value
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or_else(|| Value::String(value.into())),
        "boolean" => parse_boolean(value)
            .map(Value::Bool)
            .unwrap_or_else(|| Value::String(value.into())),
        _ => Value::String(value.into()),
    }
}

fn missing_message(name: &str) -> std::result::Result<&'static str, String> {
    match name {
        "string" => Ok("Invalid value"),
        "nonempty" => Ok("Expected a non-empty value"),
        "email" => Ok("Invalid email address"),
        "url" => Ok("Invalid URL"),
        "uuid" => Ok("Invalid UUID"),
        "number" => Ok("Expected a number"),
        "integer" => Ok("Expected an integer"),
        "boolean" => Ok("Expected a boolean"),
        "json" => Ok("Expected valid JSON"),
        "pem" => Ok("Expected a valid PEM value"),
        "sk" => Ok("Expected a valid sk secret"),
        "bearer" => Ok("Expected a valid bearer token"),
        other => Err(format!("Unknown Fatima model: {other}")),
    }
}

fn parse_boolean(value: &str) -> Option<bool> {
    match value.to_lowercase().as_str() {
        "true" | "1" | "yes" | "on" | "y" | "enabled" => Some(true),
        "false" | "0" | "no" | "off" | "n" | "disabled" => Some(false),
        _ => None,
    }
}

fn is_email(value: &str) -> bool {
    let Some((left, right)) = value.split_once('@') else {
        return false;
    };
    !left.is_empty() && right.contains('.') && !right.starts_with('.') && !right.ends_with('.')
}

fn is_url(value: &str) -> bool {
    let Some((scheme, rest)) = value.split_once("://") else {
        return false;
    };
    matches!(scheme, "http" | "https") && !rest.is_empty()
}

fn is_uuid(value: &str) -> bool {
    let parts = value.split('-').collect::<Vec<_>>();
    parts.len() == 5
        && parts
            .iter()
            .zip([8, 4, 4, 4, 12])
            .all(|(part, len)| part.len() == len && part.chars().all(|c| c.is_ascii_hexdigit()))
}
