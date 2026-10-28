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
#[ignore]
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

    let vault = FatimaVault::unlock_with_password("fixture-password", Some(options.clone())).unwrap();
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
    vault.set_secret("development", "B", "second", None).unwrap();
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
    let index_of = |id: &str| secrets.iter().position(|secret| secret["id"] == id).unwrap();
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

    let mut vault = FatimaVault::unlock_with_password("secret-password", Some(options.clone())).unwrap();
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

    let mut vault = FatimaVault::unlock_with_password("secret-password", Some(options.clone())).unwrap();
    vault.rename_environment("development", "dev").unwrap();
    vault.save().unwrap();

    let keyed = FatimaVault::authenticate_with_user_key(&generated.key, Some(options)).unwrap();
    let secrets = keyed.get_secrets(Some("dev")).unwrap();
    assert_eq!(secrets["A"], "1");
    assert_eq!(secrets["B"], "2");
}
