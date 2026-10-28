use fatima_engine::{handle_api_request, ApiRequest};
use serde_json::json;

#[test]
fn api_supports_vault_methods() {
    let temp = tempfile::tempdir().unwrap();
    let options = json!({ "cwd": temp.path().display().to_string() });

    let response = handle_api_request(ApiRequest {
        method: "vault.initialize".to_string(),
        params: json!({ "password": "secret-password", "options": options }),
    });
    assert!(response.ok, "{:?}", response.error);

    let response = handle_api_request(ApiRequest {
        method: "vault.setSecret".to_string(),
        params: json!({
            "password": "secret-password",
            "options": options,
            "environment": "development",
            "secretKey": "TOKEN",
            "value": "abc123"
        }),
    });
    assert!(response.ok, "{:?}", response.error);

    let response = handle_api_request(ApiRequest {
        method: "vault.secrets".to_string(),
        params: json!({
            "password": "secret-password",
            "options": options,
            "environment": "development"
        }),
    });
    assert!(response.ok, "{:?}", response.error);
    assert_eq!(response.data.unwrap()["TOKEN"], "abc123");
}
