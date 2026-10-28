use std::collections::BTreeMap;

use fatima_engine::vault::{
    FatimaVault, FatimaVaultSnapshot, ProjectSecretManagerSettings, SecretRecord,
};

mod text_field;

pub use crate::vault::models::{BrowseView, FormFocus, Modal, ModalFocus, Mode};
pub use text_field::TextFieldState;

pub struct VaultAppState {
    pub mode: Mode,
    pub modal: Option<Modal>,
    pub project: ProjectSecretManagerSettings,
    pub session: Option<FatimaVaultSnapshot>,
    pub vault: Option<FatimaVault>,
    pub message: Option<String>,
    pub error: Option<String>,
    pub revealed: bool,
    pub view: BrowseView,
    pub selected_environment: String,
    pub selected_index: usize,
    pub matrix_row: usize,
    pub matrix_column: usize,
    pub command_cursor: usize,
    pub modal_focus: ModalFocus,
    pub field_key: TextFieldState,
    pub field_value: TextFieldState,
    pub field_name: TextFieldState,
    pub field_path: TextFieldState,
    pub field_password: TextFieldState,
    pub field_confirm_password: TextFieldState,
    pub user_key_environment_cursor: usize,
    pub user_key_environments: Vec<String>,
    pub output_environment_cursor: usize,
    pub output_format_cursor: usize,
    pub generated_user_key: Option<String>,
    pub user_key_copied: bool,
    pub editing_secret_id: Option<String>,
    pub password: TextFieldState,
    pub confirm_password: TextFieldState,
    pub form_focus: FormFocus,
    pub should_quit: bool,
}

impl VaultAppState {
    pub fn new(project: ProjectSecretManagerSettings) -> Self {
        Self {
            selected_environment: project.default_environment.clone(),
            mode: Mode::Loading,
            modal: None,
            project,
            session: None,
            vault: None,
            message: None,
            error: None,
            revealed: false,
            view: BrowseView::Environment,
            selected_index: 0,
            matrix_row: 0,
            matrix_column: 0,
            command_cursor: 0,
            modal_focus: ModalFocus::Key,
            field_key: TextFieldState::default(),
            field_value: TextFieldState::default(),
            field_name: TextFieldState::default(),
            field_path: TextFieldState::default(),
            field_password: TextFieldState::default(),
            field_confirm_password: TextFieldState::default(),
            user_key_environment_cursor: 0,
            user_key_environments: Vec::new(),
            output_environment_cursor: 0,
            output_format_cursor: 0,
            generated_user_key: None,
            user_key_copied: false,
            editing_secret_id: None,
            password: TextFieldState::default(),
            confirm_password: TextFieldState::default(),
            form_focus: FormFocus::Password,
            should_quit: false,
        }
    }

    pub fn environment_list(&self) -> Vec<String> {
        self.session
            .as_ref()
            .map(|session| session.config.environments.clone())
            .unwrap_or_else(|| self.project.environments.clone())
    }

    pub fn vault_map(&self) -> BTreeMap<String, Vec<SecretRecord>> {
        let mut map = BTreeMap::new();
        let Some(session) = &self.session else {
            return map;
        };
        for environment in &session.config.environments {
            map.insert(
                environment.clone(),
                self.secrets_for_environment(environment),
            );
        }
        map
    }

    pub fn selected_secrets(&self) -> Vec<SecretRecord> {
        self.secrets_for_environment(&self.selected_environment)
    }

    pub fn total_secret_count(&self) -> usize {
        self.session
            .as_ref()
            .map(|session| session.secrets.len())
            .unwrap_or(0)
    }

    pub fn matrix_keys(&self) -> Vec<String> {
        self.session
            .as_ref()
            .map(|session| {
                session
                    .secrets
                    .iter()
                    .map(|secret| secret.key.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn focused_environment(&self) -> String {
        if self.view == BrowseView::Matrix {
            self.environment_list()
                .get(self.matrix_column)
                .cloned()
                .unwrap_or_else(|| self.selected_environment.clone())
        } else {
            self.selected_environment.clone()
        }
    }

    pub fn active_secret(&self) -> Option<SecretRecord> {
        if self.view == BrowseView::Matrix {
            return self.active_matrix_secret();
        }
        let secrets = self.selected_secrets();
        secrets.get(self.selected_index).cloned()
    }

    fn active_matrix_secret(&self) -> Option<SecretRecord> {
        self.session.as_ref()?.secrets.get(self.matrix_row).cloned()
    }

    pub fn active_matrix_key(&self) -> Option<String> {
        self.matrix_keys().get(self.matrix_row).cloned()
    }

    pub fn secret_key_exists(&self, key: &str) -> bool {
        self.session
            .as_ref()
            .map(|session| session.secrets.iter().any(|secret| secret.key == key))
            .unwrap_or(false)
    }

    pub fn secret_key_for_id(&self, id: &str) -> Option<String> {
        self.session
            .as_ref()?
            .secrets
            .iter()
            .find(|secret| secret.id == id)
            .map(|secret| secret.key.clone())
    }

    pub fn active_matrix_value(&self) -> Option<String> {
        let environments = self.environment_list();
        let environment = environments.get(self.matrix_column)?;
        self.active_matrix_secret()?
            .values
            .get(environment)
            .cloned()
    }

    fn secrets_for_environment(&self, environment: &str) -> Vec<SecretRecord> {
        self.session
            .as_ref()
            .map(|session| {
                session
                    .secrets
                    .iter()
                    .filter(|secret| secret.values.contains_key(environment))
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn reset_modal_fields(&mut self) {
        self.field_key.clear();
        self.field_value.clear();
        self.field_name.clear();
        self.field_path.clear();
        self.field_password.clear();
        self.field_confirm_password.clear();
        self.user_key_environment_cursor = 0;
        self.user_key_environments.clear();
        self.output_environment_cursor = 0;
        self.output_format_cursor = 0;
        self.generated_user_key = None;
        self.user_key_copied = false;
        self.editing_secret_id = None;
        self.modal_focus = ModalFocus::Key;
    }
}
