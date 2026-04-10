//! `h33 domains` — list all substrate registry domain identifiers.

use anyhow::Result;
use colored::Colorize;
use serde_json::Value;

const MANIFEST_URL: &str = "https://h33.ai/.well-known/h33-agent-manifest.json";

pub async fn run() -> Result<()> {
    let res = reqwest::Client::new()
        .get(MANIFEST_URL)
        .header("User-Agent", concat!("h33-cli/", env!("CARGO_PKG_VERSION")))
        .send()
        .await?;
    if !res.status().is_success() {
        anyhow::bail!("fetching manifest: {}", res.status());
    }
    let manifest: Value = res.json().await?;
    let assignments = manifest
        .pointer("/domain_registry/assignments")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    println!();
    println!(
        "{} {} domain identifiers",
        "H33 Substrate Registry —".bold(),
        assignments.len().to_string().bold()
    );
    println!();
    for entry in &assignments {
        let id = entry.get("id").and_then(|v| v.as_str()).unwrap_or("?");
        let name = entry.get("name").and_then(|v| v.as_str()).unwrap_or("?");
        let use_desc = entry.get("use").and_then(|v| v.as_str()).unwrap_or("");
        println!(
            "  {}  {:<30} {}",
            id.cyan(),
            name,
            use_desc.bright_black()
        );
    }
    println!();
    Ok(())
}
