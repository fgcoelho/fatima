use crate::env::Secrets;
use crate::models::{generator_type, generator_wrap, model_spec};
use crate::{FatimaConfig, FatimaError, Result};

pub struct GeneratedSpec {
    pub key: String,
    pub ty: &'static str,
    pub expression: String,
}

pub fn resolve_specs(
    config: &FatimaConfig,
    loaded: &Secrets,
    language: &str,
) -> Result<Vec<GeneratedSpec>> {
    loaded
        .keys()
        .map(|key| {
            let raw = match language {
                "typescript" => format!("process.env[{}]!", json(key)),
                _ => format!("process.env[{}]", json(key)),
            };
            let Some(model_config) = config.model.get(key) else {
                return Ok(GeneratedSpec {
                    key: key.clone(),
                    ty: "string",
                    expression: raw,
                });
            };
            let model = model_spec(key, model_config);
            let wrap = generator_wrap(model.name).ok_or_else(|| {
                FatimaError::message(format!("Unknown Fatima model: {}", model.name))
            })?;
            let mut expression = wrap.replace("$1", &raw).replace("$key", &json(key));
            if let Some(values) = model
                .args
                .and_then(|args| args.get("values"))
                .and_then(|values| serde_json::to_string(values).ok())
            {
                expression = format!("oneOf({expression}, {}, {values})", json(key));
            }
            Ok(GeneratedSpec {
                key: key.clone(),
                ty: generator_type(model.name, language).unwrap_or("string"),
                expression,
            })
        })
        .collect()
}

pub fn json(value: &str) -> String {
    serde_json::to_string(value).unwrap()
}
