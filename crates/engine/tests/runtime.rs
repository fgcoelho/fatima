use std::fs;

use fatima_engine::{Fatima, FatimaConfig};

#[test]
fn loads_local_provider_from_config_folder_and_tracks_loaded_env() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join(".env"), "PORT=3000\nNODE_ENV=development\n").unwrap();
    let config_path = dir.path().join("fatima.json");
    fs::write(
        &config_path,
        r#"{
            "generator": "typescript",
            "environment": "'{env}' ?? 'development'",
            "providers": {
                "development": [{ "provider": "local", "file": ".env" }]
            },
            "model": { "PORT": "integer" }
        }"#
        .replace("'{env}'", "{env:NODE_ENV}"),
    )
    .unwrap();

    let fatima = Fatima::from_config(FatimaConfig::load(&config_path).unwrap());
    let loaded = fatima.load_environment(Some("development"), false).unwrap();

    assert_eq!(loaded.environment, "development");
    assert_eq!(loaded.providers_used, 1);
    assert_eq!(loaded.loaded_env.get("PORT"), Some(&"3000".to_string()));
    assert!(fatima.validate("development").is_ok());
}

#[test]
fn supports_local_provider_file_arrays() {
    let dir = tempfile::tempdir().unwrap();
    fs::write(dir.path().join("one.env"), "A=1\n").unwrap();
    fs::write(dir.path().join("two.env"), "B=2\n").unwrap();
    let config_path = dir.path().join("fatima.json");
    fs::write(
        &config_path,
        r#"{
            "generator": "typescript",
            "providers": {
                "development": [{ "provider": "local", "files": ["one.env", "two.env"] }]
            }
        }"#,
    )
    .unwrap();

    let fatima = Fatima::from_config(FatimaConfig::load(&config_path).unwrap());
    let secrets = fatima.secrets("development").unwrap();

    assert_eq!(secrets.get("A"), Some(&"1".to_string()));
    assert_eq!(secrets.get("B"), Some(&"2".to_string()));
}
