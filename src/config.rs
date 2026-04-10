//! Config loading from .env files and environment variables.

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
        if trimmed.is_empty() || trimmed.starts_with('#') {
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

/// Look up a value in environment first, then fall back to .env and .env.local.
fn env_or_file(key: &str) -> Option<String> {
    if let Ok(v) = std::env::var(key) {
        if !v.is_empty() {
            return Some(v);
        }
    }
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

pub fn require_api_key() -> Result<String> {
    api_key().ok_or_else(|| {
        anyhow::anyhow!(
            "H33_API_KEY not found. Run 'h33 signup' to get one, then add it to .env."
        )
    })
}

pub fn require_agent_token() -> Result<String> {
    agent_token().ok_or_else(|| {
        anyhow::anyhow!(
            "H33_AGENT_TOKEN not found. Run 'h33 mint' to mint one, then export it or add it to .env."
        )
    })
}
