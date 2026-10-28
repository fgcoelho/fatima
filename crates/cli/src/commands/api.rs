use std::io::{self, Read};

use fatima_engine::{handle_api_request, ApiRequest};
use miette::{IntoDiagnostic, Result};

pub fn run(request: Option<String>) -> Result<()> {
    let input = match request {
        Some(request) => request,
        None => {
            let mut input = String::new();
            io::stdin().read_to_string(&mut input).into_diagnostic()?;
            input
        }
    };
    let request: ApiRequest = serde_json::from_str(&input).into_diagnostic()?;
    let response = handle_api_request(request);
    println!("{}", serde_json::to_string(&response).into_diagnostic()?);

    if response.ok {
        Ok(())
    } else {
        std::process::exit(1);
    }
}
