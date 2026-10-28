use std::collections::BTreeMap;

use fatima_engine::vault::SecretRecord;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Widget};

use crate::vault::theme;
use crate::vault::ui::atoms::panel::inset;
use crate::vault::ui::atoms::text::truncate;

const SELECTOR_WIDTH: u16 = 2;
const COUNT_WIDTH: u16 = 4;
const SECRET_KEY_WIDTH: u16 = 30;
const MATRIX_KEY_WIDTH: usize = 22;
const MATRIX_MIN_COLUMN_WIDTH: usize = 12;

pub fn mask_value(value: &str) -> String {
    "•".repeat(value.len().clamp(8, 16))
}

pub fn visible_window(total: usize, selected: usize, max_visible: usize) -> (usize, usize) {
    let visible_count = total.min(max_visible).max(1);
    let safe = selected.min(total.saturating_sub(1));
    let start = safe
        .saturating_sub(visible_count / 2)
        .min(total.saturating_sub(visible_count));
    (start, start + visible_count)
}

pub fn render_environment_tabs(
    area: Rect,
    buffer: &mut Buffer,
    environments: &[String],
    selected: &str,
    counts: &BTreeMap<String, usize>,
    dimmed: bool,
) {
    let row_style = if dimmed {
        theme::dim_surface()
    } else {
        theme::surface()
    };
    for (index, environment) in environments.iter().enumerate() {
        if index as u16 >= area.height {
            break;
        }
        let active = environment == selected;
        let count = counts.get(environment).copied().unwrap_or(0);
        let row = Rect {
            y: area.y + index as u16,
            height: 1,
            ..area
        };
        render_environment_row(inset(row), buffer, environment, count, active, row_style);
    }
}

pub fn render_secret_table(
    area: Rect,
    buffer: &mut Buffer,
    environment: &str,
    secrets: &[SecretRecord],
    selected_index: usize,
    revealed: bool,
    dimmed: bool,
) {
    if area.height == 0 {
        return;
    }
    let row_style = if dimmed {
        theme::dim_surface()
    } else {
        theme::surface()
    };
    let header = inset(Rect { height: 1, ..area });
    render_secret_table_cells(
        header,
        buffer,
        "",
        "KEY",
        "VALUE",
        theme::faint(),
        theme::faint(),
    );

    if secrets.is_empty() {
        Paragraph::new(Line::from(vec![Span::styled(
            "No secrets yet. Press 'a' to add your first secret, or 'i' to import a .env file.",
            theme::muted(),
        )]))
        .style(row_style)
        .render(
            inset(Rect {
                y: area.y + 2,
                height: 1,
                ..area
            }),
            buffer,
        );
        return;
    }

    let max_rows = area.height.saturating_sub(1) as usize;
    let (start, end) = visible_window(secrets.len(), selected_index, max_rows);
    for (offset, secret) in secrets[start..end].iter().enumerate() {
        let row_index = start + offset;
        let active = row_index == selected_index;
        let raw_value = secret.values.get(environment).cloned().unwrap_or_default();
        let value = if revealed {
            raw_value.clone()
        } else {
            mask_value(&raw_value)
        };
        let row = inset(Rect {
            y: area.y + 1 + offset as u16,
            height: 1,
            ..area
        });
        let key_style = if active {
            theme::bold()
        } else {
            theme::muted()
        };
        let value_style = if active { theme::fg() } else { theme::muted() };
        let selector = if active { "› " } else { "  " };
        render_secret_table_cells(
            row,
            buffer,
            selector,
            &secret.key,
            &value,
            key_style,
            value_style,
        );
    }
}

fn render_environment_row(
    area: Rect,
    buffer: &mut Buffer,
    environment: &str,
    count: usize,
    active: bool,
    _row_style: ratatui::style::Style,
) {
    let [selector, name, count_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(SELECTOR_WIDTH),
            Constraint::Min(0),
            Constraint::Length(COUNT_WIDTH),
        ])
        .areas(area);
    Paragraph::new(if active { "›" } else { "" })
        .style(if active { theme::fg() } else { theme::muted() })
        .render(selector, buffer);
    Paragraph::new(truncate(environment, name.width as usize))
        .style(if active {
            theme::bold()
        } else {
            theme::muted()
        })
        .render(name, buffer);
    Paragraph::new(format!(
        "{:>width$}",
        count,
        width = count_area.width as usize
    ))
    .style(theme::faint())
    .render(count_area, buffer);
}

fn render_secret_table_cells(
    area: Rect,
    buffer: &mut Buffer,
    selector: &str,
    key: &str,
    value: &str,
    key_style: ratatui::style::Style,
    value_style: ratatui::style::Style,
) {
    let key_width = SECRET_KEY_WIDTH.min(area.width.saturating_sub(SELECTOR_WIDTH + 12));
    let [selector_area, key_area, value_area] = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(SELECTOR_WIDTH),
            Constraint::Length(key_width),
            Constraint::Min(0),
        ])
        .areas(area);
    Paragraph::new(selector.to_string())
        .style(key_style)
        .render(selector_area, buffer);
    Paragraph::new(truncate(key, key_area.width as usize))
        .style(key_style)
        .render(key_area, buffer);
    Paragraph::new(truncate(value, value_area.width as usize))
        .style(value_style)
        .render(value_area, buffer);
}

pub struct MatrixSelection {
    pub row: usize,
    pub column: usize,
    pub revealed: bool,
    pub dimmed: bool,
}

pub fn render_secret_matrix(
    area: Rect,
    buffer: &mut Buffer,
    environments: &[String],
    vault: &BTreeMap<String, Vec<SecretRecord>>,
    keys: &[String],
    selection: MatrixSelection,
) {
    let MatrixSelection {
        row: selected_row,
        column: selected_column,
        revealed,
        dimmed,
    } = selection;
    let _row_style = if dimmed {
        theme::dim_surface()
    } else {
        theme::surface()
    };
    let content = inset(area);
    let available = content.width as usize;
    let fixed_width = SELECTOR_WIDTH as usize + MATRIX_KEY_WIDTH;
    let visible_columns = environments
        .len()
        .max(1)
        .min(available.saturating_sub(fixed_width) / MATRIX_MIN_COLUMN_WIDTH.max(1))
        .max(1);
    let (column_start, column_end) =
        visible_window(environments.len(), selected_column, visible_columns);
    let visible_envs = &environments[column_start..column_end.min(environments.len())];
    let column_width = MATRIX_MIN_COLUMN_WIDTH
        .max(available.saturating_sub(fixed_width) / visible_envs.len().max(1));
    let header_area = Rect {
        height: 1,
        ..content
    };
    let key_area = matrix_key_area(header_area);
    Paragraph::new("KEY")
        .style(theme::faint())
        .render(key_area, buffer);
    for (offset, env) in visible_envs.iter().enumerate() {
        let active = column_start + offset == selected_column;
        let area = matrix_column_area(header_area, offset, column_width as u16);
        Paragraph::new(truncate(&env.to_uppercase(), area.width as usize))
            .style(if active {
                theme::bold()
            } else {
                theme::faint()
            })
            .render(area, buffer);
    }

    let max_rows = area.height.saturating_sub(1) as usize;
    let (start, end) = visible_window(keys.len(), selected_row, max_rows);
    for (offset, key) in keys[start..end.min(keys.len())].iter().enumerate() {
        let row_index = start + offset;
        let row_active = row_index == selected_row;
        let row = Rect {
            y: content.y + 1 + offset as u16,
            height: 1,
            ..content
        };
        Paragraph::new(if row_active { "›" } else { "" })
            .style(if row_active {
                theme::fg()
            } else {
                theme::muted()
            })
            .render(matrix_selector_area(row), buffer);
        Paragraph::new(truncate(key, MATRIX_KEY_WIDTH))
            .style(if row_active {
                theme::bold()
            } else {
                theme::muted()
            })
            .render(matrix_key_area(row), buffer);
        for (env_offset, env) in visible_envs.iter().enumerate() {
            let column = column_start + env_offset;
            let secret = vault
                .get(env)
                .and_then(|items| items.iter().find(|secret| secret.key == *key));
            let content = secret
                .and_then(|secret| secret.values.get(env))
                .map(|value| {
                    if revealed {
                        value.clone()
                    } else {
                        mask_value(value)
                    }
                })
                .unwrap_or_else(|| "-".to_string());
            let active = row_active && column == selected_column;
            let area = matrix_column_area(row, env_offset, column_width as u16);
            Paragraph::new(truncate(&content, area.width as usize))
                .style(if active { theme::fg() } else { theme::muted() })
                .render(area, buffer);
        }
    }
}

fn matrix_selector_area(area: Rect) -> Rect {
    Rect {
        width: SELECTOR_WIDTH.min(area.width),
        ..area
    }
}

fn matrix_key_area(area: Rect) -> Rect {
    let x = area.x.saturating_add(SELECTOR_WIDTH.min(area.width));
    let width = (MATRIX_KEY_WIDTH as u16).min(area.width.saturating_sub(SELECTOR_WIDTH));
    Rect { x, width, ..area }
}

fn matrix_column_area(area: Rect, index: usize, width: u16) -> Rect {
    let start = area
        .x
        .saturating_add(SELECTOR_WIDTH)
        .saturating_add(MATRIX_KEY_WIDTH as u16)
        .saturating_add(index as u16 * width);
    let available = area.x.saturating_add(area.width).saturating_sub(start);
    Rect {
        x: start,
        width: width.min(available),
        ..area
    }
}
