use fatima_engine::vault::{FatimaVault, VaultOptions};
use fatima_engine::{Fatima, FatimaError, ProviderConfig};
use serde_json::json;

#[test]
fn vault_password_and_user_key_flow() {
    let temp = tempfile::tempdir().unwrap();
    let options = VaultOptions {
        cwd: Some(temp.path().display().to_string()),
        store_path: None,
    };

    assert!(!FatimaVault::has_store(Some(options.clone())));
    let mut vault = FatimaVault::initialize("secret-password", Some(options.clone())).unwrap();
    assert!(FatimaVault::has_store(Some(options.clone())));

    vault
        .set_secret(
            "development",
            "DATABASE_URL",
            "https://database.example.com",
            None,
        )
        .unwrap();
    let database_url_id = vault.snapshot().unwrap().secrets[0].id.clone();
    vault
        .set_secret(
            "production",
            "DATABASE_URL",
            "https://prod.example.com",
            Some(&database_url_id),
        )
        .unwrap();
    let generated = vault
        .generate_user_key("app", vec!["development".to_string()])
        .unwrap();
    vault.save().unwrap();

    let store_path = temp.path().join(".fatima/vault.json");
    let store: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(store_path).unwrap()).unwrap();
    assert_eq!(store["version"], 2);
    assert!(store.get("data").is_none());
    assert_eq!(store["kdf"]["algorithm"], "argon2id");
    assert_eq!(store["kdf"]["memoryKib"], 65536);
    assert_eq!(store["encryption"]["algorithm"], "xchacha20-poly1305");
    assert!(store["account"]["wrappedAccountKey"].get("nonce").is_some());
    assert!(store["environments"][0]["grants"][0]
        .get("sealedKey")
        .is_some());
    assert!(store["secrets"][0].get("key").is_none());
    assert!(store["secrets"][0]["keys"]["development"]
        .get("nonce")
        .is_some());
    assert!(store["secrets"][0]["values"]["development"]
        .get("nonce")
        .is_some());
    assert!(store["secrets"][0]["values"]["production"]
        .get("nonce")
        .is_some());
    assert_eq!(store["userKeys"][0]["scheme"], "v2");
    assert!(
        store["userKeys"][0]["wrappedEnvironmentSecrets"]["development"]
            .get("nonce")
            .is_some()
    );
    assert!(
        store["userKeys"][0]["wrappedUserEnvironmentSecrets"]["development"]
            .get("byAccountKey")
            .is_some()
    );
    assert!(
        store["userKeys"][0]["wrappedUserEnvironmentSecrets"]["development"]
            .get("byUserKey")
            .is_some()
    );
    assert!(store["userKeys"][0]["wrappedEnvironmentSecrets"]
        .get("production")
        .is_none());

    let snapshot = vault.snapshot().unwrap();
    assert_eq!(snapshot.secrets.len(), 1);
    assert_eq!(
        snapshot.secrets[0].values["development"],
        "https://database.example.com"
    );
    assert_eq!(
        snapshot.secrets[0].values["production"],
        "https://prod.example.com"
    );

    let unlocked =
        FatimaVault::unlock_with_password("secret-password", Some(options.clone())).unwrap();
    assert_eq!(
        unlocked.get_secrets(Some("development")).unwrap()["DATABASE_URL"],
        "https://database.example.com"
    );
    assert_eq!(
        unlocked.get_secrets(Some("production")).unwrap()["DATABASE_URL"],
        "https://prod.example.com"
    );

    let mut keyed =
        FatimaVault::authenticate_with_user_key(&generated.key, Some(options.clone())).unwrap();
    assert_eq!(keyed.list_environments().unwrap(), vec!["development"]);
    assert_eq!(
        keyed.get_secrets(Some("development")).unwrap()["DATABASE_URL"],
        "https://database.example.com"
    );
    keyed
        .set_user_secret("development", "PERSONAL_TOKEN", "user-only")
        .unwrap();
    let keyed =
        FatimaVault::authenticate_with_user_key(&generated.key, Some(options.clone())).unwrap();
    assert_eq!(
        keyed.get_secrets(Some("development")).unwrap()["PERSONAL_TOKEN"],
        "user-only"
    );
    let store: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(temp.path().join(".fatima/vault.json")).unwrap(),
    )
    .unwrap();
    assert!(store["userSecrets"][0].get("key").is_none());
    assert!(store["userSecrets"][0]["keys"]["development"]
        .get("nonce")
        .is_some());
    assert!(keyed.get_secrets(Some("production")).is_err());
}

#[test]
fn fatima_vault_provider_loads_from_user_key() {
    let temp = tempfile::tempdir().unwrap();
    let options = VaultOptions {
        cwd: Some(temp.path().display().to_string()),
        store_path: None,
    };
    let mut vault = FatimaVault::initialize("secret-password", Some(options)).unwrap();
    vault
        .set_secret("development", "TOKEN", "abc123", None)
        .unwrap();
    let generated = vault
        .generate_user_key("app", vec!["development".to_string()])
        .unwrap();
    vault.save().unwrap();

    let mut provider = ProviderConfig::new("fatima-vault");
    provider
        .options
        .insert("key".to_string(), json!(generated.key));
    provider
        .options
        .insert("cwd".to_string(), json!(temp.path().display().to_string()));

    let fatima = Fatima::new().add_provider("development", provider);
    let loaded = fatima.load_environment(Some("development"), false).unwrap();
    assert_eq!(loaded.loaded_env["TOKEN"], "abc123");
}

#[test]
fn vault_rejects_duplicate_global_secret_keys() {
    let temp = tempfile::tempdir().unwrap();
    let options = VaultOptions {
        cwd: Some(temp.path().display().to_string()),
        store_path: None,
    };
    let mut vault = FatimaVault::initialize("secret-password", Some(options)).unwrap();
    vault
        .set_secret("development", "TOKEN", "abc123", None)
        .unwrap();

    let error = vault
        .set_secret("production", "TOKEN", "def456", None)
        .unwrap_err();

    assert!(error.to_string().contains("already exists"));
}

#[test]
fn v1_fixture_opens_with_its_password() {
    let temp = tempfile::tempdir().unwrap();
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v1");
    std::fs::create_dir_all(temp.path().join(".fatima")).unwrap();
    std::fs::copy(
        fixture.join(".fatima/vault.json"),
        temp.path().join(".fatima/vault.json"),
    )
    .unwrap();
    let user_key = std::fs::read_to_string(fixture.join("user-key.txt")).unwrap();
    let options = VaultOptions {
        cwd: Some(temp.path().display().to_string()),
        store_path: None,
    };

    let vault =
        FatimaVault::unlock_with_password("fixture-password", Some(options.clone())).unwrap();
    let snapshot = vault.snapshot().unwrap();
    let value = |key: &str, environment: &str| {
        snapshot
            .secrets
            .iter()
            .find(|secret| secret.key == key)
            .unwrap()
            .values[environment]
            .clone()
    };
    assert_eq!(value("DATABASE_URL", "development"), "postgres://dev");
    assert_eq!(value("DATABASE_URL", "production"), "postgres://prod");
    assert_eq!(value("API_TOKEN", "development"), "dev-token");

    let keyed = FatimaVault::authenticate_with_user_key(user_key.trim(), Some(options)).unwrap();
    let secrets = keyed.get_secrets(Some("development")).unwrap();
    assert_eq!(secrets["DATABASE_URL"], "postgres://dev");
    assert_eq!(secrets["API_TOKEN"], "dev-token");
    assert_eq!(secrets["DEBUG"], "1");
}

fn temp_options(temp: &tempfile::TempDir) -> VaultOptions {
    VaultOptions {
        cwd: Some(temp.path().display().to_string()),
        store_path: None,
    }
}

fn read_vault_json(temp: &tempfile::TempDir) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(temp.path().join(".fatima/vault.json")).unwrap())
        .unwrap()
}

#[test]
fn v2_wrong_password_is_wrong_master_password() {
    let temp = tempfile::tempdir().unwrap();
    let options = temp_options(&temp);
    let mut vault = FatimaVault::initialize("secret-password", Some(options.clone())).unwrap();
    vault.save().unwrap();

    let error = FatimaVault::unlock_with_password("other", Some(options)).unwrap_err();

    assert!(matches!(error, FatimaError::WrongMasterPassword));
}

#[test]
fn v2_moved_ciphertext_fails_to_decrypt() {
    let temp = tempfile::tempdir().unwrap();
    let options = temp_options(&temp);
    let mut vault = FatimaVault::initialize("secret-password", Some(options.clone())).unwrap();
    vault.set_secret("development", "A", "first", None).unwrap();
    vault
        .set_secret("development", "B", "second", None)
        .unwrap();
    vault.save().unwrap();

    let mut store = read_vault_json(&temp);
    let secrets = store["secrets"].as_array().unwrap().clone();
    let snapshot = vault.snapshot().unwrap();
    let id_of = |key: &str| {
        snapshot
            .secrets
            .iter()
            .find(|secret| secret.key == key)
            .unwrap()
            .id
            .clone()
    };
    let index_of = |id: &str| {
        secrets
            .iter()
            .position(|secret| secret["id"] == id)
            .unwrap()
    };
    let source = secrets[index_of(&id_of("A"))]["values"]["development"].clone();
    store["secrets"][index_of(&id_of("B"))]["values"]["development"] = source;
    std::fs::write(
        temp.path().join(".fatima/vault.json"),
        serde_json::to_string_pretty(&store).unwrap(),
    )
    .unwrap();

    let error = FatimaVault::unlock_with_password("secret-password", Some(options)).unwrap_err();

    assert!(matches!(error, FatimaError::Decrypt));
}

#[test]
fn v2_change_password_rewraps_only_the_account_key() {
    let temp = tempfile::tempdir().unwrap();
    let options = temp_options(&temp);
    let mut vault = FatimaVault::initialize("secret-password", Some(options.clone())).unwrap();
    vault
        .set_secret("development", "TOKEN", "abc123", None)
        .unwrap();
    let generated = vault
        .generate_user_key("app", vec!["development".to_string()])
        .unwrap();
    vault.save().unwrap();
    let mut keyed =
        FatimaVault::authenticate_with_user_key(&generated.key, Some(options.clone())).unwrap();
    keyed
        .set_user_secret("development", "PERSONAL", "user-only")
        .unwrap();
    let before = read_vault_json(&temp);

    let mut vault =
        FatimaVault::unlock_with_password("secret-password", Some(options.clone())).unwrap();
    vault.change_password("new-password").unwrap();
    vault.save().unwrap();
    let after = read_vault_json(&temp);

    assert!(matches!(
        FatimaVault::unlock_with_password("secret-password", Some(options.clone())),
        Err(FatimaError::WrongMasterPassword)
    ));
    let reopened =
        FatimaVault::unlock_with_password("new-password", Some(options.clone())).unwrap();
    assert_eq!(
        reopened.get_secrets(Some("development")).unwrap()["TOKEN"],
        "abc123"
    );
    let keyed = FatimaVault::authenticate_with_user_key(&generated.key, Some(options)).unwrap();
    assert_eq!(
        keyed.get_secrets(Some("development")).unwrap()["PERSONAL"],
        "user-only"
    );
    for index in 0..before["environments"].as_array().unwrap().len() {
        assert_eq!(
            before["environments"][index]["grants"][0]["sealedKey"],
            after["environments"][index]["grants"][0]["sealedKey"]
        );
    }
    assert_ne!(
        before["account"]["wrappedAccountKey"],
        after["account"]["wrappedAccountKey"]
    );
}

#[test]
fn v2_rename_environment_keeps_user_key_access() {
    let temp = tempfile::tempdir().unwrap();
    let options = temp_options(&temp);
    let mut vault = FatimaVault::initialize("secret-password", Some(options.clone())).unwrap();
    vault.set_secret("development", "A", "1", None).unwrap();
    let generated = vault
        .generate_user_key("ci", vec!["development".to_string()])
        .unwrap();
    vault.save().unwrap();
    let mut keyed =
        FatimaVault::authenticate_with_user_key(&generated.key, Some(options.clone())).unwrap();
    keyed.set_user_secret("development", "B", "2").unwrap();

    let mut vault =
        FatimaVault::unlock_with_password("secret-password", Some(options.clone())).unwrap();
    vault.rename_environment("development", "dev").unwrap();
    vault.save().unwrap();

    let keyed = FatimaVault::authenticate_with_user_key(&generated.key, Some(options)).unwrap();
    let secrets = keyed.get_secrets(Some("dev")).unwrap();
    assert_eq!(secrets["A"], "1");
    assert_eq!(secrets["B"], "2");
}

#[test]
fn v2_recreated_environment_survives_save() {
    let temp = tempfile::tempdir().unwrap();
    let options = temp_options(&temp);
    let mut vault = FatimaVault::initialize("secret-password", Some(options.clone())).unwrap();
    vault.delete_environment("staging").unwrap();
    vault.create_environment("staging").unwrap();
    vault.set_secret("staging", "X", "1", None).unwrap();
    vault.save().unwrap();

    let reopened = FatimaVault::unlock_with_password("secret-password", Some(options)).unwrap();

    assert_eq!(reopened.get_secrets(Some("staging")).unwrap()["X"], "1");
}

#[test]
fn v2_renamed_then_recreated_environment_survives_save() {
    let temp = tempfile::tempdir().unwrap();
    let options = temp_options(&temp);
    let mut vault = FatimaVault::initialize("secret-password", Some(options.clone())).unwrap();
    vault.rename_environment("production", "prod").unwrap();
    vault.create_environment("production").unwrap();
    vault.set_secret("prod", "X", "1", None).unwrap();
    vault.set_secret("production", "Y", "2", None).unwrap();
    vault.save().unwrap();

    let reopened = FatimaVault::unlock_with_password("secret-password", Some(options)).unwrap();

    assert_eq!(reopened.get_secrets(Some("prod")).unwrap()["X"], "1");
    assert_eq!(reopened.get_secrets(Some("production")).unwrap()["Y"], "2");
}

#[test]
fn v2_out_of_range_kdf_params_are_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let options = temp_options(&temp);
    let mut vault = FatimaVault::initialize("secret-password", Some(options.clone())).unwrap();
    vault.save().unwrap();
    let mut store = read_vault_json(&temp);
    store["kdf"]["memoryKib"] = json!(4194304);
    std::fs::write(
        temp.path().join(".fatima/vault.json"),
        serde_json::to_string_pretty(&store).unwrap(),
    )
    .unwrap();

    let error = FatimaVault::unlock_with_password("secret-password", Some(options)).unwrap_err();

    assert!(matches!(error, FatimaError::InvalidKdfParams));
}

fn fixture_options(temp: &tempfile::TempDir) -> (VaultOptions, String) {
    let fixture = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/v1");
    std::fs::create_dir_all(temp.path().join(".fatima")).unwrap();
    std::fs::copy(
        fixture.join(".fatima/vault.json"),
        temp.path().join(".fatima/vault.json"),
    )
    .unwrap();
    let user_key = std::fs::read_to_string(fixture.join("user-key.txt")).unwrap();
    (temp_options(temp), user_key.trim().to_string())
}

fn assert_fixture_values(vault: &FatimaVault) {
    let snapshot = vault.snapshot().unwrap();
    let value = |key: &str, environment: &str| {
        snapshot
            .secrets
            .iter()
            .find(|secret| secret.key == key)
            .unwrap()
            .values[environment]
            .clone()
    };
    assert_eq!(value("DATABASE_URL", "development"), "postgres://dev");
    assert_eq!(value("DATABASE_URL", "production"), "postgres://prod");
    assert_eq!(value("API_TOKEN", "development"), "dev-token");
}

#[test]
fn v1_fixture_upgrades_on_save() {
    let temp = tempfile::tempdir().unwrap();
    let (options, _) = fixture_options(&temp);

    let mut vault =
        FatimaVault::unlock_with_password("fixture-password", Some(options.clone())).unwrap();
    vault.save().unwrap();

    let store = read_vault_json(&temp);
    assert_eq!(store["version"], 2);
    assert_eq!(store["kdf"]["algorithm"], "argon2id");
    let reopened = FatimaVault::unlock_with_password("fixture-password", Some(options)).unwrap();
    assert_fixture_values(&reopened);
}

#[test]
fn v1_user_key_survives_the_upgrade() {
    let temp = tempfile::tempdir().unwrap();
    let (options, user_key) = fixture_options(&temp);

    let mut vault =
        FatimaVault::unlock_with_password("fixture-password", Some(options.clone())).unwrap();
    vault.save().unwrap();

    let keyed = FatimaVault::authenticate_with_user_key(&user_key, Some(options)).unwrap();
    let secrets = keyed.get_secrets(Some("development")).unwrap();
    assert_eq!(secrets["DATABASE_URL"], "postgres://dev");
    assert_eq!(secrets["API_TOKEN"], "dev-token");
    assert_eq!(secrets["DEBUG"], "1");
    let store = read_vault_json(&temp);
    let record = &store["userKeys"][0];
    assert_eq!(record["scheme"], "v1");
    let fixture: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/v1/.fatima/vault.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let original = &fixture["userKeys"][0];
    assert_ne!(record["secretHash"], original["secretHash"]);
    assert_eq!(
        record["wrappedEnvironmentSecrets"],
        original["wrappedEnvironmentSecrets"]
    );
    let wrapped = original["wrappedUserEnvironmentSecrets"]
        .as_object()
        .unwrap();
    assert!(!wrapped.is_empty());
    for (environment, blobs) in wrapped {
        assert_eq!(
            record["wrappedUserEnvironmentSecrets"][environment]["byUserKey"],
            blobs["byUserKey"]
        );
    }
    assert!(
        store["userKeys"][0]["wrappedUserEnvironmentSecrets"]["development"]
            .get("byAccountKey")
            .is_some()
    );
}

#[test]
fn v1_wrong_password_is_wrong_master_password() {
    let temp = tempfile::tempdir().unwrap();
    let (options, _) = fixture_options(&temp);

    let error = FatimaVault::unlock_with_password("other", Some(options)).unwrap_err();

    assert!(matches!(error, FatimaError::WrongMasterPassword));
}

#[test]
fn v1_vault_without_environments_is_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let (options, _) = fixture_options(&temp);
    let path = temp.path().join(".fatima/vault.json");
    let mut store: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    store["environments"] = json!([]);
    std::fs::write(&path, serde_json::to_string(&store).unwrap()).unwrap();

    let error = FatimaVault::unlock_with_password("fixture-password", Some(options)).unwrap_err();

    assert!(error.to_string().contains("no environments"));
}

#[test]
fn user_key_created_after_upgrade_is_v2() {
    let temp = tempfile::tempdir().unwrap();
    let (options, fixture_key) = fixture_options(&temp);

    let mut vault =
        FatimaVault::unlock_with_password("fixture-password", Some(options.clone())).unwrap();
    vault.save().unwrap();
    let generated = vault
        .generate_user_key("deploy", vec!["production".to_string()])
        .unwrap();
    vault.save().unwrap();

    let store = read_vault_json(&temp);
    let record = store["userKeys"]
        .as_array()
        .unwrap()
        .iter()
        .find(|record| record["id"] == generated.record.id.as_str())
        .unwrap();
    assert_eq!(record["scheme"], "v2");
    let keyed =
        FatimaVault::authenticate_with_user_key(&generated.key, Some(options.clone())).unwrap();
    assert_eq!(
        keyed.get_secrets(Some("production")).unwrap()["DATABASE_URL"],
        "postgres://prod"
    );
    let fixture_keyed =
        FatimaVault::authenticate_with_user_key(&fixture_key, Some(options)).unwrap();
    let secrets = fixture_keyed.get_secrets(Some("development")).unwrap();
    assert_eq!(secrets["DEBUG"], "1");
    assert_eq!(secrets["DATABASE_URL"], "postgres://dev");
}

#[test]
fn v1_user_key_writes_a_user_secret_before_the_upgrade() {
    let temp = tempfile::tempdir().unwrap();
    let (options, user_key) = fixture_options(&temp);

    let mut keyed =
        FatimaVault::authenticate_with_user_key(&user_key, Some(options.clone())).unwrap();
    keyed
        .set_user_secret("development", "LOG", "debug")
        .unwrap();

    assert_eq!(read_vault_json(&temp)["version"], 1);
    let keyed = FatimaVault::authenticate_with_user_key(&user_key, Some(options.clone())).unwrap();
    assert_eq!(
        keyed.get_secrets(Some("development")).unwrap()["LOG"],
        "debug"
    );

    let mut vault =
        FatimaVault::unlock_with_password("fixture-password", Some(options.clone())).unwrap();
    assert_fixture_values(&vault);
    vault.save().unwrap();

    assert_eq!(read_vault_json(&temp)["version"], 2);
    let keyed = FatimaVault::authenticate_with_user_key(&user_key, Some(options)).unwrap();
    let secrets = keyed.get_secrets(Some("development")).unwrap();
    assert_eq!(secrets["LOG"], "debug");
    assert_eq!(secrets["DEBUG"], "1");
}

fn fixture_user_key_hash() -> serde_json::Value {
    let fixture: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/v1/.fatima/vault.json"),
        )
        .unwrap(),
    )
    .unwrap();
    fixture["userKeys"][0]["secretHash"].clone()
}

#[test]
fn v1_user_key_hash_is_not_its_wrapping_key_after_upgrade() {
    let temp = tempfile::tempdir().unwrap();
    let (options, user_key) = fixture_options(&temp);
    let original = fixture_user_key_hash();

    let mut vault =
        FatimaVault::unlock_with_password("fixture-password", Some(options.clone())).unwrap();
    vault.save().unwrap();

    let store = read_vault_json(&temp);
    assert_eq!(store["version"], 2);
    assert_ne!(store["userKeys"][0]["secretHash"], original);
    let keyed = FatimaVault::authenticate_with_user_key(&user_key, Some(options)).unwrap();
    let secrets = keyed.get_secrets(Some("development")).unwrap();
    assert_eq!(secrets["DATABASE_URL"], "postgres://dev");
    assert_eq!(secrets["API_TOKEN"], "dev-token");
    assert_eq!(secrets["DEBUG"], "1");
}

#[test]
fn v1_user_key_writes_after_the_upgrade() {
    let temp = tempfile::tempdir().unwrap();
    let (options, user_key) = fixture_options(&temp);

    let mut vault =
        FatimaVault::unlock_with_password("fixture-password", Some(options.clone())).unwrap();
    vault.save().unwrap();

    let mut keyed =
        FatimaVault::authenticate_with_user_key(&user_key, Some(options.clone())).unwrap();
    keyed
        .set_user_secret("development", "LOG", "debug")
        .unwrap();

    let keyed = FatimaVault::authenticate_with_user_key(&user_key, Some(options)).unwrap();
    let secrets = keyed.get_secrets(Some("development")).unwrap();
    assert_eq!(secrets["LOG"], "debug");
    assert_eq!(secrets["DEBUG"], "1");
}

#[test]
fn v1_user_key_survives_a_rename_after_the_upgrade() {
    let temp = tempfile::tempdir().unwrap();
    let (options, user_key) = fixture_options(&temp);

    let mut vault =
        FatimaVault::unlock_with_password("fixture-password", Some(options.clone())).unwrap();
    vault.rename_environment("development", "dev").unwrap();
    vault.save().unwrap();

    let keyed = FatimaVault::authenticate_with_user_key(&user_key, Some(options)).unwrap();
    let secrets = keyed.get_secrets(Some("dev")).unwrap();
    assert_eq!(secrets["DATABASE_URL"], "postgres://dev");
    assert_eq!(secrets["DEBUG"], "1");
}

#[test]
fn debug_output_hides_key_material() {
    let temp = tempfile::tempdir().unwrap();
    let (options, user_key) = fixture_options(&temp);

    let mut vault =
        FatimaVault::unlock_with_password("fixture-password", Some(options.clone())).unwrap();
    let keyed = FatimaVault::authenticate_with_user_key(&user_key, Some(options)).unwrap();

    for output in [format!("{vault:?}"), format!("{keyed:?}")] {
        assert!(output.ends_with(".. }"), "{output}");
        assert!(!output.contains(&user_key), "{output}");
        assert!(!output.contains("account_key"), "{output}");
        assert!(!output.contains("postgres://"), "{output}");
    }
    let generated = vault
        .generate_user_key("debug", vec!["development".to_string()])
        .unwrap();
    assert!(!format!("{generated:?}").contains(&generated.key));
}
