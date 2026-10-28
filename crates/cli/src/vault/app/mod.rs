use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use fatima_engine::vault::VaultOptions;
use miette::{miette, IntoDiagnostic, Result};
use ratatui::Frame;

use crate::vault::components::shells::render_app_shell;
use crate::vault::input::handle_key;
use crate::vault::models::Mode;
use crate::vault::runtime::terminal;
use crate::vault::runtime::VaultRuntime;
use crate::vault::state::VaultAppState;
use crate::vault::ui::atoms::toast::render_toast;
use crate::vault::views;

pub fn run() -> Result<()> {
    let runtime = VaultRuntime::new(VaultOptions::default());
    let project = runtime
        .get_project()
        .map_err(|error| miette!(error.to_string()))?;
    let mut state = VaultAppState::new(project);
    state.mode = if runtime.has_store() {
        Mode::Unlock
    } else {
        Mode::Onboarding
    };

    if let Some(password) = fatima_engine::FatimaVault::resolve_default_password(None) {
        match runtime.unlock(&password) {
            Ok((vault, snapshot)) => {
                state.project = snapshot.project.clone();
                state.selected_environment = snapshot.config.default_environment.clone();
                state.vault = Some(vault);
                state.session = Some(snapshot);
                state.mode = Mode::Vault;
            }
            Err(error) => state.error = Some(error.to_string()),
        }
    }

    let mut terminal = terminal::enter().into_diagnostic()?;
    let result = run_loop(&mut terminal, &runtime, &mut state);
    terminal::leave(&mut terminal).into_diagnostic()?;
    result
}

fn run_loop(
    terminal: &mut terminal::VaultTerminal,
    runtime: &VaultRuntime,
    state: &mut VaultAppState,
) -> Result<()> {
    while !state.should_quit {
        terminal
            .draw(|frame| render(frame, state))
            .into_diagnostic()?;
        if event::poll(Duration::from_millis(80)).into_diagnostic()? {
            if let Event::Key(key) = event::read().into_diagnostic()? {
                if key.kind == KeyEventKind::Release {
                    continue;
                }
                handle_key(state, runtime, key)?;
            }
        }
    }
    Ok(())
}

fn render(frame: &mut Frame, state: &VaultAppState) {
    let area = frame.area();
    let shell = render_app_shell(area, frame.buffer_mut(), state);

    match state.mode {
        Mode::Loading => {}
        Mode::Onboarding => views::onboarding::render(shell.body, frame.buffer_mut(), state),
        Mode::Unlock => views::unlock::render(shell.body, frame.buffer_mut(), state),
        Mode::Vault => views::vault::render(shell.body, frame.buffer_mut(), state),
    }

    if state.mode != Mode::Vault {
        if let Some(error) = &state.error {
            render_toast(shell.body, frame.buffer_mut(), error, true);
        } else if let Some(message) = &state.message {
            render_toast(shell.body, frame.buffer_mut(), message, false);
        }
    }

    match state.modal {
        Some(crate::vault::state::Modal::CommandPalette) => {
            views::command_palette::render(shell.body, frame.buffer_mut(), state)
        }
        Some(_) => crate::vault::modals::render(shell.body, frame.buffer_mut(), state),
        None => {}
    }
}
