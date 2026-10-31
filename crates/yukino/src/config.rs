#![deny(unsafe_code)]

use std::fs;
use std::path::PathBuf;

pub fn global_config_path() -> PathBuf {
    if let Some(custom_path) = std::env::var_os("YUKINO_CONFIG_PATH") {
        return PathBuf::from(custom_path);
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return PathBuf::from(appdata)
                .join("yukino-lang")
                .join("config.json");
        }
        if let Some(user_profile) = std::env::var_os("USERPROFILE") {
            return PathBuf::from(user_profile)
                .join("AppData")
                .join("Roaming")
                .join("yukino-lang")
                .join("config.json");
        }
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join("Library")
                .join("Application Support")
                .join("yukino-lang")
                .join("config.json");
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        if let Some(xdg) = std::env::var_os("XDG_CONFIG_HOME") {
            return PathBuf::from(xdg).join("yukino-lang").join("config.json");
        }
        if let Some(home) = std::env::var_os("HOME") {
            return PathBuf::from(home)
                .join(".config")
                .join("yukino-lang")
                .join("config.json");
        }
    }

    PathBuf::from("config.json")
}

pub fn parse_cthreads_from_json(json: &str) -> Option<usize> {
    let key = "\"cthreads\"";
    let key_idx = json.find(key)?;
    let after_key = &json[key_idx + key.len()..];
    let colon_idx = after_key.find(':')?;
    let after_colon = after_key[colon_idx + 1..].trim_start();

    // extract contiguous numeric digits
    let digits: String = after_colon
        .chars()
        .take_while(char::is_ascii_digit)
        .collect();

    if digits.is_empty() {
        return None;
    }

    digits.parse::<usize>().ok()
}

pub fn parse_minibar_from_json(json: &str) -> Option<bool> {
    let val: serde_json::Value = serde_json::from_str(json).ok()?;
    let item = val.get("minibar")?;
    if let Some(b) = item.as_bool() {
        Some(b)
    } else if let Some(s) = item.as_str() {
        s.parse::<bool>().ok()
    } else {
        None
    }
}

pub fn parse_noanim_from_json(json: &str) -> Option<bool> {
    let val: serde_json::Value = serde_json::from_str(json).ok()?;
    let item = val.get("noanim")?;
    if let Some(b) = item.as_bool() {
        Some(b)
    } else if let Some(s) = item.as_str() {
        s.parse::<bool>().ok()
    } else {
        None
    }
}

pub fn parse_full_lib_log_from_json(json: &str) -> Option<bool> {
    let val: serde_json::Value = serde_json::from_str(json).ok()?;
    let item = val.get("full-lib-log").or_else(|| val.get("full_lib_log"))?;
    if let Some(b) = item.as_bool() {
        Some(b)
    } else if let Some(s) = item.as_str() {
        s.parse::<bool>().ok()
    } else {
        None
    }
}

pub fn parse_copy_output_path_from_json(json: &str) -> Option<bool> {
    let val: serde_json::Value = serde_json::from_str(json).ok()?;
    let item = val
        .get("copy_output_path")
        .or_else(|| val.get("copy-output-path"))?;
    if let Some(b) = item.as_bool() {
        Some(b)
    } else if let Some(s) = item.as_str() {
        s.parse::<bool>().ok()
    } else {
        None
    }
}

pub fn load_global_cthreads() -> Option<usize> {
    let path = global_config_path();
    let content = fs::read_to_string(path).ok()?;
    parse_cthreads_from_json(&content)
}

pub fn load_global_minibar() -> bool {
    let path = global_config_path();
    let Ok(content) = fs::read_to_string(path) else {
        return false;
    };
    parse_minibar_from_json(&content).unwrap_or(false)
}

pub fn load_global_noanim() -> bool {
    let path = global_config_path();
    let Ok(content) = fs::read_to_string(path) else {
        return false;
    };
    parse_noanim_from_json(&content).unwrap_or(false)
}

pub fn load_global_full_lib_log() -> bool {
    let path = global_config_path();
    let Ok(content) = fs::read_to_string(path) else {
        return false;
    };
    parse_full_lib_log_from_json(&content).unwrap_or(false)
}

pub fn load_global_copy_output_path() -> bool {
    let path = global_config_path();
    let Ok(content) = fs::read_to_string(path) else {
        return false;
    };
    parse_copy_output_path_from_json(&content).unwrap_or(false)
}

pub fn parse_macrotimeout_from_json(json: &str) -> Option<u64> {
    let val: serde_json::Value = serde_json::from_str(json).ok()?;
    let item = val.get("macrotimeout")?;
    if let Some(n) = item.as_u64() {
        Some(n)
    } else if let Some(s) = item.as_str() {
        s.parse::<u64>().ok()
    } else {
        None
    }
}

pub fn load_global_macrotimeout() -> u64 {
    let path = global_config_path();
    let Ok(content) = fs::read_to_string(path) else {
        return 10;
    };
    parse_macrotimeout_from_json(&content).unwrap_or(10)
}

pub fn num_cpus() -> usize {
    std::thread::available_parallelism().map_or(1, std::num::NonZero::get)
}

pub fn default_pool_threads() -> usize {
    load_global_cthreads().unwrap_or_else(num_cpus).max(1)
}

pub fn resolve_pool_threads(cli_threads: Option<usize>) -> usize {
    cli_threads.map_or_else(default_pool_threads, |n| n.max(1))
}

fn load_or_init_json(path: &std::path::Path) -> serde_json::Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_else(|| serde_json::json!({ "cthreads": 4 }))
}

pub fn set_config_value_at(path: &std::path::Path, key: &str, value: &str) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut root = load_or_init_json(path);
    let obj = root
        .as_object_mut()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "not an object"))?;

    let parsed_val = if let Ok(n) = value.parse::<i64>() {
        serde_json::Value::Number(n.into())
    } else if let Ok(b) = value.parse::<bool>() {
        serde_json::Value::Bool(b)
    } else {
        serde_json::Value::String(value.to_string())
    };

    obj.insert(key.to_string(), parsed_val);
    let pretty = serde_json::to_string_pretty(&root)
        .map_err(std::io::Error::other)?;
    fs::write(path, pretty)?;
    Ok(())
}

pub fn get_config_value_at(path: &std::path::Path, key: &str) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let val: serde_json::Value = serde_json::from_str(&content).ok()?;
    let item = val.get(key)?;
    match item {
        serde_json::Value::String(s) => Some(s.clone()),
        other => Some(other.to_string()),
    }
}

pub fn list_config_at(path: &std::path::Path) -> Vec<(String, String)> {
    let mut list = Vec::new();
    let Ok(content) = fs::read_to_string(path) else {
        return vec![("cthreads".to_string(), "4".to_string())];
    };
    let Ok(serde_json::Value::Object(map)) = serde_json::from_str(&content) else {
        return vec![("cthreads".to_string(), "4".to_string())];
    };
    for (k, v) in map {
        let val_str = match v {
            serde_json::Value::String(s) => s,
            other => other.to_string(),
        };
        list.push((k, val_str));
    }
    list.sort_by(|a, b| a.0.cmp(&b.0));
    list
}

pub fn set_global_config_value(key: &str, value: &str) -> Result<(), std::io::Error> {
    set_config_value_at(&global_config_path(), key, value)
}

pub fn get_global_config_value(key: &str) -> Option<String> {
    get_config_value_at(&global_config_path(), key)
}

pub fn list_global_config() -> Vec<(String, String)> {
    list_config_at(&global_config_path())
}

pub fn set_cthreads(count: usize) -> Result<(), std::io::Error> {
    set_global_config_value("cthreads", &count.to_string())
}

pub fn set_macrotimeout(secs: u64) -> Result<(), std::io::Error> {
    set_global_config_value("macrotimeout", &secs.to_string())
}

pub fn set_copy_output_path(enabled: bool) -> Result<(), std::io::Error> {
    set_global_config_value("copy_output_path", &enabled.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_copy_output_path_json() {
        assert_eq!(
            parse_copy_output_path_from_json(r#"{"copy_output_path": true}"#),
            Some(true)
        );
        assert_eq!(
            parse_copy_output_path_from_json(r#"{"copy_output_path": false}"#),
            Some(false)
        );
        assert_eq!(
            parse_copy_output_path_from_json(r#"{"copy_output_path": "true"}"#),
            Some(true)
        );
        assert_eq!(
            parse_copy_output_path_from_json(r#"{"copy-output-path": true}"#),
            Some(true)
        );
        assert_eq!(parse_copy_output_path_from_json(r#"{"other": 1}"#), None);
    }

    #[test]
    fn test_parse_cthreads_valid_json() {
        let json = r#"{"cthreads": 8}"#;
        assert_eq!(parse_cthreads_from_json(json), Some(8));
    }

    #[test]
    fn test_parse_cthreads_with_spaces_and_newlines() {
        let json = "{\n  \"name\": \"yukino\",\n  \"cthreads\" :  12 ,\n  \"version\": 1\n}";
        assert_eq!(parse_cthreads_from_json(json), Some(12));
    }

    #[test]
    fn test_parse_cthreads_missing() {
        let json = r#"{"other": 4}"#;
        assert_eq!(parse_cthreads_from_json(json), None);
    }

    #[test]
    fn test_parse_minibar_json() {
        assert_eq!(parse_minibar_from_json(r#"{"minibar": true}"#), Some(true));
        assert_eq!(parse_minibar_from_json(r#"{"minibar": false}"#), Some(false));
        assert_eq!(parse_minibar_from_json(r#"{"minibar": "true"}"#), Some(true));
        assert_eq!(parse_minibar_from_json(r#"{"other": 1}"#), None);
    }

    #[test]
    fn test_parse_noanim_json() {
        assert_eq!(parse_noanim_from_json(r#"{"noanim": true}"#), Some(true));
        assert_eq!(parse_noanim_from_json(r#"{"noanim": false}"#), Some(false));
        assert_eq!(parse_noanim_from_json(r#"{"noanim": "true"}"#), Some(true));
        assert_eq!(parse_noanim_from_json(r#"{"other": 1}"#), None);
    }

    #[test]
    fn test_parse_full_lib_log_json() {
        assert_eq!(parse_full_lib_log_from_json(r#"{"full-lib-log": true}"#), Some(true));
        assert_eq!(parse_full_lib_log_from_json(r#"{"full-lib-log": false}"#), Some(false));
        assert_eq!(parse_full_lib_log_from_json(r#"{"full-lib-log": "true"}"#), Some(true));
        assert_eq!(parse_full_lib_log_from_json(r#"{"full-lib-log": "false"}"#), Some(false));
        assert_eq!(parse_full_lib_log_from_json(r#"{"full_lib_log": true}"#), Some(true));
        assert_eq!(parse_full_lib_log_from_json(r#"{"other": 1}"#), None);
    }

    #[test]
    fn test_parse_macrotimeout_json() {
        assert_eq!(parse_macrotimeout_from_json(r#"{"macrotimeout": 20}"#), Some(20));
        assert_eq!(parse_macrotimeout_from_json(r#"{"macrotimeout": "15"}"#), Some(15));
        assert_eq!(parse_macrotimeout_from_json(r#"{"macrotimeout": 0}"#), Some(0));
        assert_eq!(parse_macrotimeout_from_json(r#"{"other": 1}"#), None);
    }

    #[test]
    fn test_num_cpus_positive() {
        assert!(num_cpus() >= 1);
    }

    #[test]
    fn test_resolve_pool_threads_cli_override() {
        assert_eq!(resolve_pool_threads(Some(16)), 16);
        assert_eq!(resolve_pool_threads(Some(0)), 1);
    }

    #[test]
    fn test_global_config_set_get_list() {
        let tmp = std::env::temp_dir().join(format!("test_cfg_{}.json", std::process::id()));

        set_config_value_at(&tmp, "cthreads", "8").expect("set cthreads");
        set_config_value_at(&tmp, "custom_opt", "true").expect("set custom_opt");

        assert_eq!(get_config_value_at(&tmp, "cthreads"), Some("8".to_string()));
        assert_eq!(get_config_value_at(&tmp, "custom_opt"), Some("true".to_string()));
        assert_eq!(get_config_value_at(&tmp, "nonexistent"), None);

        let list = list_config_at(&tmp);
        assert!(list.iter().any(|(k, v)| k == "cthreads" && v == "8"));
        assert!(list.iter().any(|(k, v)| k == "custom_opt" && v == "true"));

        let _ = fs::remove_file(tmp);
    }
}
