pub fn generator_type(name: &str, language: &str) -> Option<&'static str> {
    match (name, language) {
        ("number" | "integer", "typescript") => Some("number"),
        ("boolean", "typescript") => Some("boolean"),
        ("json", "typescript") => Some("unknown"),
        (_, "typescript") => Some("string"),
        _ => None,
    }
}

pub fn generator_wrap(name: &str) -> Option<&'static str> {
    match name {
        "string" => Some("string($1, $key)"),
        "nonempty" => Some("nonEmpty($1, $key)"),
        "email" => Some("email($1, $key)"),
        "url" => Some("url($1, $key)"),
        "uuid" => Some("uuid($1, $key)"),
        "number" => Some("number($1, $key)"),
        "integer" => Some("integer($1, $key)"),
        "boolean" => Some("boolean($1, $key)"),
        "json" => Some("json($1, $key)"),
        "pem" => Some("pem($1, $key)"),
        "sk" => Some("secretKey($1, $key)"),
        "bearer" => Some("bearer($1, $key)"),
        _ => None,
    }
}
