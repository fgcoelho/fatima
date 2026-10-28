use std::path::PathBuf;

use fatima_engine::{Fatima, FatimaConfig};
use miette::Result;

use crate::ui::atoms;
use crate::ui::spinner::Spinner;

pub fn run(environment: Option<String>, config: PathBuf) -> Result<()> {
    let spinner = Spinner::start("Validating environment");
    let result = (|| {
        let config = FatimaConfig::load(config)?;
        let fatima = Fatima::from_config(config);
        let loaded = fatima.load_environment(environment.as_deref(), false)?;
        fatima.validate(loaded.environment)?;
        Ok::<_, Box<dyn std::error::Error>>(())
    })();

    match result {
        Ok(()) => {
            spinner.clear();
            println!("{}", atoms::success("Environment is valid."));
            Ok(())
        }
        Err(error) => {
            spinner.clear();
            eprintln!(
                "{}",
                atoms::error_block("Validation failed", error.to_string())
            );
            Err(miette::miette!(""))
        }
    }
}
