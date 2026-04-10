//! `h33 verify <anchor>` — verify a substrate anchor by ID.

use crate::{client::H33Client, config, output};
use anyhow::Result;
use colored::Colorize;
use serde_json::json;

pub async fn run(api_base: &str, anchor_id: &str) -> Result<()> {
    let token = config::require_agent_token()?;
    let client = H33Client::new(api_base)?;
    let body = json!({"anchor_id": anchor_id});
    let result = client.post_json("/v1/substrate/verify", &token, body).await?;

    println!();
    if result.get("valid").and_then(|v| v.as_bool()).unwrap_or(false) {
        let domain = result.get("domain").and_then(|v| v.as_str()).unwrap_or("?");
        let age = result.get("age_ms").and_then(|v| v.as_i64()).unwrap_or(0);
        output::ok(&format!("Anchor verified · domain={} · age={}ms", domain, age));
    } else {
        output::err("Anchor invalid");
    }
    if let Some(created_at) = result.get("created_at").and_then(|v| v.as_str()) {
        println!("  {} {}", "Created:".bold(), created_at);
    }
    println!();
    Ok(())
}
