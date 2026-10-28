use std::path::PathBuf;

use fatima_engine::{generate, FatimaConfig};
use miette::Result;

use crate::ui::atoms;
use crate::ui::spinner::Spinner;

pub fn run(environment: Option<String>, config: PathBuf, strict: bool) -> Result<()> {
    let spinner = Spinner::start("Loading environment");
    let result = (|| {
        let config = FatimaConfig::load(config)?;
        generate(config, environment.as_deref(), strict)
    })();

    match result {
        Ok(output) => {
            spinner.clear();
            println!("{}", atoms::generated(display_path(&output.output_path)));
            println!(
                "{}",
                atoms::meta(format!(
                    "Environment: {} ({} vars)",
                    output.environment,
                    output.loaded_env.len()
                ))
            );
            Ok(())
        }
        Err(error) => {
            spinner.clear();
            eprintln!(
                "{}",
                atoms::error_block("Failed to generate environment", error.to_string())
            );
            Err(miette::miette!(""))
        }
    }
}

fn display_path(path: &str) -> String {
    let path = PathBuf::from(path);
    std::env::current_dir()
        .ok()
        .and_then(|cwd| path.strip_prefix(cwd).ok().map(PathBuf::from))
        .unwrap_or(path)
        .display()
        .to_string()
}
