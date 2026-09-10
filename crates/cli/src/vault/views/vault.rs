use crate::vault::components::command_bar::render_command_bar;
use crate::vault::models::BrowseView;
use crate::vault::state::VaultAppState;
use crate::vault::ui::atoms::panel::{fill, render_panel_header};
use crate::vault::ui::atoms::toast::ToastTone;
use crate::vault::view_models::vault::use_vault;
use crate::vault::view_partials::vault::{
    render_environment_tabs, render_secret_matrix, render_secret_table, MatrixSelection,
};
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub fn render(area: Rect, buffer: &mut Buffer, state: &VaultAppState) {
    let vm = use_vault(state);
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);
    let columns = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(26),
            Constraint::Length(2),
            Constraint::Min(0),
        ])
        .split(rows[0]);

    fill(
        columns[0],
        buffer,
        vm.view == BrowseView::Environment,
        vm.overlay_open,
    );
    let left_body = render_panel_header(
        columns[0],
        buffer,
        "vault",
        &format!("{} environments", vm.environment_list.len()),
        !vm.overlay_open && vm.view == BrowseView::Environment,
        vm.overlay_open,
    );
    render_environment_tabs(
        left_body,
        buffer,
        &vm.environment_list,
        &vm.focused_environment,
        &vm.secret_counts,
        vm.overlay_open,
    );

    fill(columns[2], buffer, true, vm.overlay_open);
    let title = if vm.view == BrowseView::Environment {
        vm.selected_environment.as_str()
    } else {
        "matrix"
    };
    let subtitle = if vm.view == BrowseView::Environment {
        format!("{} secrets", vm.selected_secret_count)
    } else {
        format!("{} keys", vm.matrix_key_count)
    };
    let right_body = render_panel_header(
        columns[2],
        buffer,
        title,
        &subtitle,
        !vm.overlay_open,
        vm.overlay_open,
    );
    match vm.view {
        BrowseView::Environment => render_secret_table(
            right_body,
            buffer,
            &state.selected_environment,
            &state.selected_secrets(),
            state.selected_index,
            vm.revealed,
            vm.overlay_open,
        ),
        BrowseView::Matrix => render_secret_matrix(
            right_body,
            buffer,
            &vm.environment_list,
            &state.vault_map(),
            &state.matrix_keys(),
            MatrixSelection {
                row: state.matrix_row,
                column: state.matrix_column,
                revealed: vm.revealed,
                dimmed: vm.overlay_open,
            },
        ),
    }

    render_command_bar(rows[2], buffer, toast(state));
}

fn toast(state: &VaultAppState) -> Option<(&str, ToastTone)> {
    if let Some(error) = &state.error {
        Some((error.as_str(), ToastTone::Error))
    } else if let Some(message) = &state.message {
        Some((message.as_str(), message_tone(message)))
    } else {
        None
    }
}

fn message_tone(message: &str) -> ToastTone {
    if message.to_lowercase().contains("copied") {
        ToastTone::Info
    } else {
        ToastTone::Success
    }
}
