use super::Secrets;

pub fn parse_env(content: &str) -> Secrets {
    let mut env = Secrets::new();
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    for line in normalized.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        env.insert(key.trim().to_string(), unquote(value.trim()));
    }
    env
}

fn unquote(value: &str) -> String {
    if value.len() >= 2 {
        let bytes = value.as_bytes();
        if (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'')
            || (bytes[0] == b'`' && bytes[value.len() - 1] == b'`')
        {
            let mut output = value[1..value.len() - 1].to_string();
            if bytes[0] == b'"' {
                output = output.replace("\\n", "\n").replace("\\r", "\r");
            }
            return output;
        }
    }
    value.to_string()
}
