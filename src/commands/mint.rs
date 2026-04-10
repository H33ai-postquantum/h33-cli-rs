//! `h33 mint` — mint a short-lived cka_* agent capability token.

use crate::{client::H33Client, config, output};
use anyhow::Result;
use colored::Colorize;
use serde_json::json;

pub async fn run(
    api_base: &str,
    ttl: u64,
    production: bool,
    user: &str,
    agent: &str,
) -> Result<()> {
    let api_key = config::require_api_key()?;

    if production {
        output::err(
            "Production tokens must be promoted via the dashboard. Run 'h33 mint' without --production.",
        );
        std::process::exit(1);
    }

    output::info(&format!(
        "Minting agent token (ttl={}s, sandbox=true)…",
        ttl
    ));

    let client = H33Client::new(api_base)?;
    let body = json!({
        "capabilities": ["agent:standard"],
        "ttl_seconds": ttl,
        "sandbox": true,
        "agent_identifier": agent,
        "human_user_id": user,
    });

    let result = client
        .post_json("/v1/agent_tokens", &api_key, body)
        .await?;

    let token = result.get("token").and_then(|v| v.as_str()).unwrap_or("");
    let session_id = result
        .get("session_id")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let expires_at = result
        .get("expires_at")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let caps = result
        .get("capabilities_granted")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_default();

    output::ok("Token minted");
    println!();
    println!("  {}        {}", "Token:".bold(), token.green());
    println!("  {}   {}", "Session ID:".bold(), session_id);
    println!("  {}      {}", "Expires:".bold(), expires_at);
    println!("  {} {}", "Capabilities:".bold(), caps);
    println!();
    println!("  {}", "Export to use:".bold());
    println!(
        "    {}",
        format!("export H33_AGENT_TOKEN=\"{}\"", token).bright_black()
    );
    println!();
    println!("  {}", "Or add to .env:".bold());
    println!(
        "    {}",
        format!("echo 'H33_AGENT_TOKEN={}' >> .env", token).bright_black()
    );
    println!();
    Ok(())
}
