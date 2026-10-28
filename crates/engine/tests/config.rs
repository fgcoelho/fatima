use std::collections::BTreeMap;

use fatima_engine::config::{evaluate_environment_expression, interpolate_value};

#[test]
fn evaluates_env_expression_with_fallback() {
    let env = BTreeMap::new();

    assert_eq!(
        evaluate_environment_expression("{env:NODE_ENV} ?? 'development'", &env).unwrap(),
        "development"
    );
}

#[test]
fn interpolates_nested_json_values() {
    let env = BTreeMap::from([("TOKEN".to_string(), "secret".to_string())]);
    let value = serde_json::json!({ "key": "{env:TOKEN}", "items": ["x", "{env:TOKEN}"] });

    assert_eq!(
        interpolate_value(value, &env).unwrap(),
        serde_json::json!({ "key": "secret", "items": ["x", "secret"] })
    );
}
