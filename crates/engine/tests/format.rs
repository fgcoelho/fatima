use std::collections::BTreeMap;

use fatima_engine::{format_secrets, SecretFormat};

fn secrets() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("API_KEY".to_string(), "abc123".to_string()),
        (
            "DATABASE_URL".to_string(),
            "postgres://localhost/db".to_string(),
        ),
    ])
}

#[test]
fn formats_dotenv() {
    assert_eq!(
        format_secrets(&secrets(), SecretFormat::parse("dotenv").unwrap()).unwrap(),
        "API_KEY=abc123\nDATABASE_URL=postgres://localhost/db"
    );
}

#[test]
fn formats_yml() {
    assert_eq!(
        format_secrets(&secrets(), SecretFormat::parse("yml").unwrap()).unwrap(),
        "API_KEY: abc123\nDATABASE_URL: postgres://localhost/db\n"
    );
}

#[test]
fn formats_bash_export() {
    assert_eq!(
        format_secrets(&secrets(), SecretFormat::parse("bash-export").unwrap()).unwrap(),
        "export API_KEY='abc123'\nexport DATABASE_URL='postgres://localhost/db'"
    );
}

#[test]
fn rejects_aliases() {
    assert!(SecretFormat::parse("env").is_err());
    assert!(SecretFormat::parse("yaml").is_err());
    assert!(SecretFormat::parse("shell").is_err());
}
