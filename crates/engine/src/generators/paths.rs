use std::path::Path;

pub fn sibling_public_path(file: &str) -> String {
    let path = Path::new(file);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("env");
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| format!(".{value}"))
        .unwrap_or_default();
    path.parent()
        .unwrap_or_else(|| Path::new(""))
        .join(format!("{stem}.public{extension}"))
        .display()
        .to_string()
}
