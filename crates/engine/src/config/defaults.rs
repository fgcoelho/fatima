pub fn default_generator() -> String {
    "typescript".to_string()
}

pub fn default_generator_file(generator: &str) -> &'static str {
    match generator {
        "typescript" => "env.ts",
        "javascript" => "env.js",
        "python" => "env.py",
        _ => "env",
    }
}
