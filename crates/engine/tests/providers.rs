use std::collections::BTreeMap;
use std::fs;

use fatima_engine::providers::{load_provider, parse_env};
use fatima_engine::{NativeHost, ProviderConfig};
use serde_json::json;

#[test]
fn parses_env_lines_like_fatima() {
    let env = parse_env("A=1\nexport B='two'\nC=\"hello\\nworld\"\nD=`four`\n# skip");

    assert_eq!(env.get("A"), Some(&"1".to_string()));
    assert_eq!(env.get("B"), Some(&"two".to_string()));
    assert_eq!(env.get("C"), Some(&"hello\nworld".to_string()));
    assert_eq!(env.get("D"), Some(&"four".to_string()));
}

#[test]
fn file_provider_loads_env_file() {
    let temp = tempfile::tempdir().expect("temp dir");
    fs::write(temp.path().join(".env"), "A=1\nB=two").expect("write env");

    let mut provider = ProviderConfig::new("file");
    provider.options.insert("file".to_string(), json!(".env"));
    provider.options.insert("format".to_string(), json!("env"));

    let env = futures::executor::block_on(load_provider(
        temp.path(),
        "development",
        &provider,
        &BTreeMap::new(),
        &NativeHost,
    ))
    .expect("load provider");

    assert_eq!(env.get("A"), Some(&"1".to_string()));
    assert_eq!(env.get("B"), Some(&"two".to_string()));
}

#[test]
fn file_provider_loads_multiple_files_in_order() {
    let temp = tempfile::tempdir().expect("temp dir");
    fs::write(temp.path().join(".env"), "A=1\nB=base").expect("write base env");
    fs::write(temp.path().join(".env.local"), "B=local\nC=3").expect("write local env");

    let mut provider = ProviderConfig::new("file");
    provider
        .options
        .insert("files".to_string(), json!([".env", ".env.local"]));

    let env = futures::executor::block_on(load_provider(
        temp.path(),
        "development",
        &provider,
        &BTreeMap::new(),
        &NativeHost,
    ))
    .expect("load provider");

    assert_eq!(env.get("A"), Some(&"1".to_string()));
    assert_eq!(env.get("B"), Some(&"local".to_string()));
    assert_eq!(env.get("C"), Some(&"3".to_string()));
}

#[test]
fn local_provider_alias_still_loads_env_file() {
    let temp = tempfile::tempdir().expect("temp dir");
    fs::write(temp.path().join(".env"), "A=1").expect("write env");

    let provider = ProviderConfig::new("local");
    let env = futures::executor::block_on(load_provider(
        temp.path(),
        "development",
        &provider,
        &BTreeMap::new(),
        &NativeHost,
    ))
    .expect("load provider");

    assert_eq!(env.get("A"), Some(&"1".to_string()));
}

#[test]
fn file_provider_rejects_unsupported_format() {
    let temp = tempfile::tempdir().expect("temp dir");
    fs::write(temp.path().join("env.json"), "{}").expect("write env json");

    let mut provider = ProviderConfig::new("file");
    provider
        .options
        .insert("file".to_string(), json!("env.json"));
    provider.options.insert("format".to_string(), json!("json"));

    let error = futures::executor::block_on(load_provider(
        temp.path(),
        "development",
        &provider,
        &BTreeMap::new(),
        &NativeHost,
    ))
    .expect_err("unsupported format should fail");

    assert!(error.to_string().contains("Supported formats: env"));
}

#[test]
fn file_provider_infers_and_rejects_unsupported_format() {
    let temp = tempfile::tempdir().expect("temp dir");
    fs::write(temp.path().join("env.json"), "{}").expect("write env json");

    let mut provider = ProviderConfig::new("file");
    provider
        .options
        .insert("file".to_string(), json!("env.json"));

    let error = futures::executor::block_on(load_provider(
        temp.path(),
        "development",
        &provider,
        &BTreeMap::new(),
        &NativeHost,
    ))
    .expect_err("unsupported inferred format should fail");

    assert!(error.to_string().contains("format `json`"));
}

#[test]
fn fatima_cloud_provider_is_explicitly_not_implemented() {
    let mut provider = ProviderConfig::new("fatima");
    provider
        .options
        .insert("source".to_string(), json!("cloud"));

    let error = futures::executor::block_on(load_provider(
        std::path::Path::new("."),
        "development",
        &provider,
        &BTreeMap::new(),
        &NativeHost,
    ))
    .expect_err("cloud source should fail");

    assert!(error.to_string().contains("not implemented yet"));
}
