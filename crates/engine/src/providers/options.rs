use std::collections::BTreeMap;

use serde_json::Value;

use crate::env::Secrets;

pub(crate) fn option_string(options: &BTreeMap<String, Value>, key: &str) -> Option<String> {
    options.get(key)?.as_str().map(ToOwned::to_owned)
}

pub(crate) fn option_string_vec(
    options: &BTreeMap<String, Value>,
    key: &str,
) -> Option<Vec<String>> {
    match options.get(key)? {
        Value::String(value) => Some(vec![value.clone()]),
        Value::Array(values) => Some(
            values
                .iter()
                .filter_map(|value| value.as_str().map(ToOwned::to_owned))
                .collect(),
        ),
        _ => None,
    }
}

pub(crate) fn option_env_string(
    options: &BTreeMap<String, Value>,
    env: &Secrets,
    option_key: &str,
    env_key: &str,
) -> Option<String> {
    option_string(options, option_key)
        .filter(|value| !value.trim().is_empty())
        .or_else(|| env.get(env_key).cloned())
        .or_else(|| std::env::var(env_key).ok())
        .filter(|value| !value.trim().is_empty())
}
