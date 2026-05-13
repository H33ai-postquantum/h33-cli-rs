//! Config loading from ~/.h33/config.toml, .env files, and environment variables.
//!
//! Priority order (highest first):
//! 1. Environment variable (H33_API_KEY, H33_AGENT_TOKEN)
//! 2. ~/.h33/config.toml (created by `h33 init`)
//! 3. .env / .env.local in current directory

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Parse a .env file into key-value pairs. Quoted values are stripped.
fn parse_env_file(path: &Path) -> Result<HashMap<String, String>> {
    let mut out = HashMap::new();
    if !path.exists() {
        return Ok(out);
    }
    let content = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('[') {
            continue;
        }
        let Some(eq) = trimmed.find('=') else { continue };
        let key = trimmed[..eq].trim().to_string();
        let mut value = trimmed[eq + 1..].trim().to_string();
        if (value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\''))
        {
            value = value[1..value.len() - 1].to_string();
        }
        out.insert(key, value);
    }
    Ok(out)
}

/// Read api_key from ~/.h33/config.toml if it exists.
fn from_h33_config(key: &str) -> Option<String> {
    let home = dirs::home_dir()?;
    let config_path = home.join(".h33").join("config.toml");
    if !config_path.exists() {
        return None;
    }
    // Simple TOML parsing — look for `api_key = "value"` under [auth]
    let content = fs::read_to_string(&config_path).ok()?;
    let toml_key = match key {
        "H33_API_KEY" => "api_key",
        "H33_AGENT_TOKEN" => "agent_token",
        "H33_API_BASE" => "api_base",
        _ => return None,
    };
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with(toml_key) {
            if let Some(eq) = trimmed.find('=') {
                let mut value = trimmed[eq + 1..].trim().to_string();
                if (value.starts_with('"') && value.ends_with('"'))
                    || (value.starts_with('\'') && value.ends_with('\''))
                {
                    value = value[1..value.len() - 1].to_string();
                }
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
    }
    None
}

/// Look up a value: env → ~/.h33/config.toml → .env → .env.local
fn env_or_file(key: &str) -> Option<String> {
    // 1. Environment variable
    if let Ok(v) = std::env::var(key) {
        if !v.is_empty() {
            return Some(v);
        }
    }
    // 2. ~/.h33/config.toml (from `h33 init`)
    if let Some(v) = from_h33_config(key) {
        return Some(v);
    }
    // 3. .env / .env.local in current directory
    for f in [".env", ".env.local"] {
        if let Ok(map) = parse_env_file(Path::new(f)) {
            if let Some(v) = map.get(key) {
                if !v.is_empty() {
                    return Some(v.clone());
                }
            }
        }
    }
    None
}

pub fn api_key() -> Option<String> {
    env_or_file("H33_API_KEY")
}

pub fn agent_token() -> Option<String> {
    env_or_file("H33_AGENT_TOKEN")
}

pub fn api_base() -> Option<String> {
    env_or_file("H33_API_BASE")
}

pub fn require_api_key() -> Result<String> {
    api_key().ok_or_else(|| {
        anyhow::anyhow!(
            "H33_API_KEY not found. Run 'h33 init' to set up your account."
        )
    })
}

pub fn require_agent_token() -> Result<String> {
    agent_token().ok_or_else(|| {
        anyhow::anyhow!(
            "H33_AGENT_TOKEN not found. Run 'h33 mint' to mint one."
        )
    })
}
