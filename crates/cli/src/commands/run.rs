use std::path::PathBuf;
use std::process::Command;

use fatima_engine::{Fatima, FatimaConfig};
use miette::Result;

use crate::ui::atoms;

pub fn run(
    environment: Option<String>,
    config: PathBuf,
    process_env: bool,
    command: Vec<String>,
) -> Result<()> {
    if command.is_empty() {
        return Err(miette::miette!(
            "Missing command. Example: fatima node index.js"
        ));
    }

    let config = FatimaConfig::load(config).map_err(|error| miette::miette!(error.to_string()))?;
    let fatima = Fatima::from_config(config);
    let loaded = fatima
        .load_environment(environment.as_deref(), process_env)
        .map_err(|error| miette::miette!(error.to_string()))?;

    eprintln!(
        "{}",
        atoms::meta(format!(
            "Loaded {} vars for {}.",
            loaded.loaded_env.len(),
            loaded.environment
        ))
    );
    eprintln!("{}", atoms::meta(format!("Command: {}", command.join(" "))));

    let status = Command::new(&command[0])
        .args(&command[1..])
        .envs(loaded.env)
        .status()
        .map_err(|error| miette::miette!(error.to_string()))?;

    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}
