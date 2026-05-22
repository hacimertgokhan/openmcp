use serde::Serialize;
use serde_json::{Map, Value};
use std::{
    env, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Serialize)]
struct ConfigFile {
    path: String,
    label: String,
    exists: bool,
    mcp_count: usize,
}

#[derive(Serialize)]
struct McpServer {
    name: String,
    source_path: String,
    scope: String,
    server_type: String,
    enabled: bool,
    command: Option<Vec<String>>,
    url: Option<String>,
    environment_keys: Vec<String>,
}

#[derive(Serialize)]
struct AppState {
    configs: Vec<ConfigFile>,
    servers: Vec<McpServer>,
}

#[tauri::command]
fn get_app_state() -> Result<AppState, String> {
    load_state()
}

#[tauri::command]
fn set_mcp_enabled(config_path: String, name: String, enabled: bool) -> Result<AppState, String> {
    let path = PathBuf::from(config_path);
    let mut value = read_config_or_default(&path)?;
    let entry = value
        .get_mut("mcp")
        .and_then(Value::as_object_mut)
        .and_then(|mcp| mcp.get_mut(&name))
        .and_then(Value::as_object_mut)
        .ok_or_else(|| format!("MCP not found: {name}"))?;
    entry.insert("enabled".to_string(), Value::Bool(enabled));
    write_config(&path, &value)?;
    load_state()
}

#[tauri::command]
fn remove_mcp(config_path: String, name: String) -> Result<AppState, String> {
    let path = PathBuf::from(config_path);
    let mut value = read_config_or_default(&path)?;
    let mcp = ensure_mcp_object(&mut value)?;
    if mcp.remove(&name).is_none() {
        return Err(format!("MCP not found: {name}"));
    }
    write_config(&path, &value)?;
    load_state()
}

#[tauri::command]
fn install_mcp(config_path: String, name: String, definition: Value) -> Result<AppState, String> {
    let path = PathBuf::from(config_path);
    upsert_mcp(&path, &name, definition)?;
    load_state()
}

#[tauri::command]
fn copy_mcp_to_project(
    source_config_path: String,
    name: String,
    project_path: String,
) -> Result<String, String> {
    let source = PathBuf::from(source_config_path);
    let source_value = read_config_or_default(&source)?;
    let definition = source_value
        .get("mcp")
        .and_then(Value::as_object)
        .and_then(|mcp| mcp.get(&name))
        .cloned()
        .ok_or_else(|| format!("MCP not found: {name}"))?;

    let target = project_config_path(&project_path)?;
    upsert_mcp(&target, &name, definition)?;
    Ok(target.display().to_string())
}

#[tauri::command]
fn install_catalog_to_project(project_path: String, name: String, definition: Value) -> Result<String, String> {
    let target = project_config_path(&project_path)?;
    upsert_mcp(&target, &name, definition)?;
    Ok(target.display().to_string())
}

fn load_state() -> Result<AppState, String> {
    let candidates = candidate_configs();
    let mut configs = Vec::new();
    let mut servers = Vec::new();

    for (path, label) in candidates {
        let exists = path.exists();
        let value = if exists { Some(read_config_or_default(&path)?) } else { None };
        let mcp_count = value
            .as_ref()
            .and_then(|json| json.get("mcp"))
            .and_then(Value::as_object)
            .map(Map::len)
            .unwrap_or(0);

        configs.push(ConfigFile {
            path: path.display().to_string(),
            label,
            exists,
            mcp_count,
        });

        if let Some(json) = value {
            if let Some(mcp) = json.get("mcp").and_then(Value::as_object) {
                for (name, definition) in mcp {
                    servers.push(to_server(name, &path, definition));
                }
            }
        }
    }

    servers.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(AppState { configs, servers })
}

fn candidate_configs() -> Vec<(PathBuf, String)> {
    let mut items = Vec::new();

    if let Ok(path) = env::var("OPENCODE_CONFIG") {
        items.push((PathBuf::from(path), "OPENCODE_CONFIG".to_string()));
    }

    if let Some(home) = dirs::home_dir() {
        items.push((home.join(".config").join("opencode").join("opencode.json"), "Global".to_string()));
        items.push((home.join(".config").join("opencode").join("opencode.jsonc"), "Global JSONC".to_string()));
        items.push((home.join(".opencode.json"), "Home".to_string()));
    }

    if let Ok(current) = env::current_dir() {
        items.push((current.join("opencode.json"), "Launch project".to_string()));
        items.push((current.join(".opencode").join("opencode.jsonc"), "Launch project JSONC".to_string()));
    }

    dedupe_paths(items)
}

fn dedupe_paths(items: Vec<(PathBuf, String)>) -> Vec<(PathBuf, String)> {
    let mut seen = Vec::<String>::new();
    let mut result = Vec::new();
    for (path, label) in items {
        let key = path.display().to_string().to_lowercase();
        if !seen.contains(&key) {
            seen.push(key);
            result.push((path, label));
        }
    }
    result
}

fn to_server(name: &str, source_path: &Path, definition: &Value) -> McpServer {
    let object = definition.as_object();
    let server_type = object
        .and_then(|item| item.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("local")
        .to_string();
    let enabled = object
        .and_then(|item| item.get("enabled"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let command = object
        .and_then(|item| item.get("command"))
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).map(redact_sensitive_text).collect());
    let url = object
        .and_then(|item| item.get("url"))
        .and_then(Value::as_str)
        .map(redact_sensitive_text);
    let environment_keys = object
        .and_then(|item| item.get("environment"))
        .and_then(Value::as_object)
        .map(|env| env.keys().cloned().collect())
        .unwrap_or_default();

    McpServer {
        name: name.to_string(),
        source_path: source_path.display().to_string(),
        scope: scope_for_path(source_path),
        server_type,
        enabled,
        command,
        url,
        environment_keys,
    }
}

fn scope_for_path(path: &Path) -> String {
    let text = path.display().to_string().replace('\\', "/").to_lowercase();
    if text.contains("/.config/opencode/") || text.ends_with("/.opencode.json") {
        "global".to_string()
    } else if text.contains("/.opencode/") || text.ends_with("/opencode.json") {
        "project".to_string()
    } else {
        "custom".to_string()
    }
}

fn read_config_or_default(path: &Path) -> Result<Value, String> {
    if !path.exists() {
        return Ok(Value::Object(Map::new()));
    }
    let content = fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let normalized = normalize_jsonc(&content);
    serde_json::from_str(&normalized).map_err(|error| format!("{}: {error}", path.display()))
}

fn normalize_jsonc(input: &str) -> String {
    remove_trailing_commas(&strip_json_comments(input))
}

fn strip_json_comments(input: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if in_string {
            output.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            output.push(ch);
            continue;
        }

        if ch == '/' {
            match chars.peek() {
                Some('/') => {
                    chars.next();
                    for next in chars.by_ref() {
                        if next == '\n' {
                            output.push('\n');
                            break;
                        }
                    }
                }
                Some('*') => {
                    chars.next();
                    let mut previous = '\0';
                    for next in chars.by_ref() {
                        if previous == '*' && next == '/' {
                            break;
                        }
                        previous = next;
                    }
                }
                _ => output.push(ch),
            }
        } else {
            output.push(ch);
        }
    }

    output
}

fn remove_trailing_commas(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut output = String::with_capacity(input.len());
    let mut index = 0;
    let mut in_string = false;
    let mut escaped = false;

    while index < chars.len() {
        let ch = chars[index];
        if in_string {
            output.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            index += 1;
            continue;
        }

        if ch == '"' {
            in_string = true;
            output.push(ch);
            index += 1;
            continue;
        }

        if ch == ',' {
            let mut lookahead = index + 1;
            while lookahead < chars.len() && chars[lookahead].is_whitespace() {
                lookahead += 1;
            }
            if lookahead < chars.len() && (chars[lookahead] == '}' || chars[lookahead] == ']') {
                index += 1;
                continue;
            }
        }

        output.push(ch);
        index += 1;
    }

    output
}

fn redact_sensitive_text(value: &str) -> String {
    let mut text = value.to_string();
    text = redact_url_userinfo(&text);
    text = redact_query_values(&text);
    text = redact_assignment_values(&text);
    text
}

fn redact_url_userinfo(value: &str) -> String {
    if let Some(scheme_index) = value.find("://") {
        let authority_start = scheme_index + 3;
        let rest = &value[authority_start..];
        let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let authority = &rest[..authority_end];
        if let Some(at_index) = authority.rfind('@') {
            let mut redacted = String::new();
            redacted.push_str(&value[..authority_start]);
            redacted.push_str("<redacted>@");
            redacted.push_str(&authority[at_index + 1..]);
            redacted.push_str(&rest[authority_end..]);
            return redacted;
        }
    }
    value.to_string()
}

fn redact_query_values(value: &str) -> String {
    let Some(query_start) = value.find('?') else {
        return value.to_string();
    };
    let mut output = String::new();
    output.push_str(&value[..=query_start]);
    let query = &value[query_start + 1..];
    let mut first = true;
    for part in query.split('&') {
        if !first {
            output.push('&');
        }
        first = false;
        if let Some((key, value_part)) = part.split_once('=') {
            if is_sensitive_key(key) {
                output.push_str(key);
                output.push_str("=<redacted>");
            } else {
                output.push_str(key);
                output.push('=');
                output.push_str(value_part);
            }
        } else {
            output.push_str(part);
        }
    }
    output
}

fn redact_assignment_values(value: &str) -> String {
    let mut text = value.to_string();
    for key in ["password", "passwd", "pwd", "token", "secret", "api_key", "apikey", "access_key"] {
        text = redact_after_key(&text, key, '=');
        text = redact_after_key(&text, key, ':');
    }
    text
}

fn redact_after_key(value: &str, key: &str, separator: char) -> String {
    let lower = value.to_lowercase();
    let pattern = format!("{key}{separator}");
    let mut cursor = 0;
    let mut output = String::new();

    while let Some(relative) = lower[cursor..].find(&pattern) {
        let start = cursor + relative;
        let value_start = start + pattern.len();
        output.push_str(&value[cursor..value_start]);
        output.push_str("<redacted>");
        let mut value_end = value_start;
        for (offset, ch) in value[value_start..].char_indices() {
            if ch == '&' || ch == ';' || ch == ',' || ch.is_whitespace() {
                break;
            }
            value_end = value_start + offset + ch.len_utf8();
        }
        cursor = value_end;
    }

    output.push_str(&value[cursor..]);
    output
}

fn is_sensitive_key(key: &str) -> bool {
    let lower = key.to_lowercase();
    ["password", "passwd", "pwd", "token", "secret", "key", "auth", "credential"]
        .iter()
        .any(|needle| lower.contains(needle))
}

fn write_config(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }

    if path.exists() {
        let backup = backup_path(path)?;
        fs::copy(path, &backup).map_err(|error| format!("backup {}: {error}", backup.display()))?;
    }

    let content = serde_json::to_string_pretty(value).map_err(|error| error.to_string())?;
    fs::write(path, format!("{content}\n")).map_err(|error| format!("{}: {error}", path.display()))
}

fn backup_path(path: &Path) -> Result<PathBuf, String> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| error.to_string())?
        .as_secs();
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("json");
    Ok(path.with_extension(format!("{extension}.bak-{timestamp}")))
}

fn ensure_mcp_object(value: &mut Value) -> Result<&mut Map<String, Value>, String> {
    if !value.is_object() {
        *value = Value::Object(Map::new());
    }
    let root = value.as_object_mut().ok_or_else(|| "Invalid config root".to_string())?;
    if !root.contains_key("mcp") || !root.get("mcp").is_some_and(Value::is_object) {
        root.insert("mcp".to_string(), Value::Object(Map::new()));
    }
    root.get_mut("mcp")
        .and_then(Value::as_object_mut)
        .ok_or_else(|| "Invalid mcp section".to_string())
}

fn upsert_mcp(path: &Path, name: &str, mut definition: Value) -> Result<(), String> {
    let mut value = read_config_or_default(path)?;
    if !definition.is_object() {
        return Err("MCP definition must be an object".to_string());
    }
    if let Some(object) = definition.as_object_mut() {
        object.entry("enabled".to_string()).or_insert(Value::Bool(true));
    }
    let mcp = ensure_mcp_object(&mut value)?;
    mcp.insert(name.to_string(), definition);
    write_config(path, &value)
}

fn project_config_path(project_path: &str) -> Result<PathBuf, String> {
    let trimmed = project_path.trim();
    if trimmed.is_empty() {
        return Err("Project path is empty".to_string());
    }
    let path = PathBuf::from(trimmed);
    let target = if path.extension().is_some_and(|ext| ext == "json" || ext == "jsonc") {
        path
    } else {
        path.join("opencode.json")
    };
    Ok(target)
}

pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            get_app_state,
            set_mcp_enabled,
            remove_mcp,
            install_mcp,
            copy_mcp_to_project,
            install_catalog_to_project
        ])
        .run(tauri::generate_context!())
        .expect("error while running OpenMCP");
}
