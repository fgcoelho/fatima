pub(crate) enum Field {
    Name,
    Value,
}

impl Field {
    fn as_str(&self) -> &'static str {
        match self {
            Field::Name => "name",
            Field::Value => "value",
        }
    }
}

pub(crate) fn account_key(project_id: &str) -> Vec<u8> {
    format!("fatima/v2/{project_id}/account-key").into_bytes()
}

pub(crate) fn private_key(project_id: &str) -> Vec<u8> {
    format!("fatima/v2/{project_id}/private-key").into_bytes()
}

pub(crate) fn user_key_environment(project_id: &str, user_key_id: &str) -> Vec<u8> {
    format!("fatima/v2/{project_id}/user-key/{user_key_id}").into_bytes()
}

pub(crate) fn user_environment(project_id: &str, user_key_id: &str) -> Vec<u8> {
    format!("fatima/v2/{project_id}/user-environment/{user_key_id}").into_bytes()
}

pub(crate) fn secret_field(
    project_id: &str,
    environment: &str,
    secret_id: &str,
    field: Field,
) -> Vec<u8> {
    format!(
        "fatima/v2/{project_id}/{environment}/{secret_id}/{}",
        field.as_str()
    )
    .into_bytes()
}

pub(crate) fn user_secret_field(
    project_id: &str,
    environment: &str,
    user_key_id: &str,
    secret_id: &str,
    field: Field,
) -> Vec<u8> {
    format!(
        "fatima/v2/{project_id}/{environment}/user/{user_key_id}/{secret_id}/{}",
        field.as_str()
    )
    .into_bytes()
}
