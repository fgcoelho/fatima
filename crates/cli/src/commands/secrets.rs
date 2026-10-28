use std::path::PathBuf;

use fatima_engine::{format_secrets, Fatima, FatimaConfig, SecretFormat};
use miette::Result;

use crate::ui::atoms;
use crate::ui::spinner::Spinner;

pub fn run(environment: Option<String>, format: String, config: PathBuf) -> Result<()> {
    let spinner = Spinner::start("Loading secrets");
    let result = (|| {
        let config = FatimaConfig::load(config)?;
        let format = SecretFormat::parse(&format)?;
        let fatima = Fatima::from_config(config);
        let loaded = fatima.load_environment(environment.as_deref(), false)?;
        let secrets = loaded.loaded_env;
        Ok::<_, Box<dyn std::error::Error>>(format_secrets(&secrets, format)?)
    })();

    match result {
        Ok(output) => {
            spinner.clear();
            println!("{output}");
            Ok(())
        }
        Err(error) => {
            spinner.clear();
            eprintln!(
                "{}",
                atoms::error_block("Failed to load secrets", error.to_string())
            );
            Err(miette::miette!(""))
        }
    }
}
