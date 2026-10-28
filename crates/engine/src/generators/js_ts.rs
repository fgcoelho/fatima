use super::spec::{json, resolve_specs};
use crate::env::Secrets;
use crate::{FatimaConfig, Result};

const TYPESCRIPT_TEMPLATE: &str = include_str!("../templates/private/env.ts");
const TYPESCRIPT_PUBLIC_TEMPLATE: &str = include_str!("../templates/public/env.ts");
const JAVASCRIPT_TEMPLATE: &str = include_str!("../templates/private/env.js");
const JAVASCRIPT_PUBLIC_TEMPLATE: &str = include_str!("../templates/public/env.js");

pub fn render_typescript(config: &FatimaConfig, loaded: &Secrets, public: bool) -> Result<String> {
    render_typescript_file(config, loaded, public)
}

pub fn render_javascript(config: &FatimaConfig, loaded: &Secrets, public: bool) -> Result<String> {
    if public {
        render_javascript_public(config, loaded)
    } else {
        render(config, loaded, public, "javascript", JAVASCRIPT_TEMPLATE)
    }
}

fn render_javascript_public(config: &FatimaConfig, loaded: &Secrets) -> Result<String> {
    let values = resolve_specs(config, loaded, "javascript")?
        .iter()
        .map(|spec| format!("\t{}: {},", json(&spec.key), spec.expression))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(JAVASCRIPT_PUBLIC_TEMPLATE.replace("\t// __PUBLIC_VALUES__", &values))
}

fn render(
    config: &FatimaConfig,
    loaded: &Secrets,
    public: bool,
    language: &str,
    template: &str,
) -> Result<String> {
    let type_name = if public { "PublicEnv" } else { "Env" };
    let export_name = if public { "publicEnv" } else { "env" };
    let cache_name = if public {
        "publicEnvCache".to_string()
    } else {
        "envCache".to_string()
    };
    let resolve_name = if public {
        "resolvePublicEnv".to_string()
    } else {
        "resolveEnv".to_string()
    };
    let specs = resolve_specs(config, loaded, language)?;

    let type_body = specs
        .iter()
        .map(|spec| format!("\t{}: {};", json(&spec.key), spec.ty))
        .collect::<Vec<_>>()
        .join("\n");
    let object_body = specs
        .iter()
        .map(|spec| format!("\t\t{}: {},", json(&spec.key), spec.expression))
        .collect::<Vec<_>>()
        .join("\n");
    let getter_body = specs
        .iter()
        .map(|spec| {
            format!(
                "\tget {}() {{ return {resolve_name}()[{}]; }},",
                json(&spec.key),
                json(&spec.key)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let exports = if language == "javascript" {
        if public {
            format!("module.exports = {{ {export_name} }};")
        } else {
            format!("module.exports = {{ {export_name}, register, registerAsync }};")
        }
    } else {
        String::new()
    };

    Ok(template
        .replace(
            "// __FATIMA_IMPORTS__",
            if language == "typescript" && !public {
                "import { readFileSync } from \"node:fs\";"
            } else {
                ""
            },
        )
        .replace(
            "// __FATIMA_REGISTER__",
            if public {
                ""
            } else if language == "typescript" {
                include_str!("../templates/register/register.ts")
            } else {
                include_str!("../templates/register/register.js")
            },
        )
        .replace("// __FATIMA_EXPORTS__", &exports)
        .replace("__TYPE_NAME__", type_name)
        .replace("__EXPORT_NAME__", export_name)
        .replace("__CACHE_NAME__", &cache_name)
        .replace("__RESOLVE_NAME__", &resolve_name)
        .replace("\t// __TYPE_BODY__", &type_body)
        .replace("\t\t// __OBJECT_BODY__", &object_body)
        .replace("\t// __GETTER_BODY__", &getter_body))
}

fn render_typescript_file(config: &FatimaConfig, loaded: &Secrets, public: bool) -> Result<String> {
    let prefix = config.public_prefix.as_deref().unwrap_or("PUBLIC_");
    let specs = resolve_specs(config, loaded, "typescript")?;
    let type_body = specs
        .iter()
        .map(|spec| format!("\t{}: {};", json(&spec.key), spec.ty))
        .collect::<Vec<_>>()
        .join("\n");

    if public {
        let values = specs
            .iter()
            .map(|spec| format!("\t{}: {},", json(&spec.key), spec.expression))
            .collect::<Vec<_>>()
            .join("\n");
        return Ok(TYPESCRIPT_PUBLIC_TEMPLATE
            .replace("\t// __TYPE_BODY__", &type_body)
            .replace("\t// __PUBLIC_VALUES__", &values));
    }

    let resolvers = specs
        .iter()
        .filter(|spec| !spec.key.starts_with(prefix))
        .map(|spec| format!("\t{}: () => {},", json(&spec.key), spec.expression))
        .collect::<Vec<_>>()
        .join("\n");

    Ok(TYPESCRIPT_TEMPLATE
        .replace(
            "// __FATIMA_IMPORTS__",
            "import { readFileSync } from \"node:fs\";",
        )
        .replace(
            "// __FATIMA_REGISTER__",
            include_str!("../templates/register/register.ts"),
        )
        .replace("__PUBLIC_PREFIX__", prefix)
        .replace("\t// __TYPE_BODY__", &type_body)
        .replace("\t// __PRIVATE_RESOLVERS__", &resolvers))
}
