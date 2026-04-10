//! `h33 audit` — read recent agent audit log entries.

use crate::{client::H33Client, config, output};
use anyhow::Result;
use colored::Colorize;

pub async fn run(api_base: &str, limit: u32) -> Result<()> {
    let api_key = config::require_api_key()?;
    let client = H33Client::new(api_base)?;
    let path = format!("/v1/audit?limit={}", limit);
    let result = client.get_json(&path, Some(&api_key)).await?;

    let entries = result
        .get("entries")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    println!();
    println!("{}", "Recent audit log entries:".bold());
    println!();

    for entry in &entries {
        let ts = entry
            .get("timestamp")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let actor = entry.get("actor").and_then(|v| v.as_str()).unwrap_or("?");
        let action = entry.get("action").and_then(|v| v.as_str()).unwrap_or("?");
        let resource = entry
            .get("resource")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let outcome = entry
            .get("outcome")
            .and_then(|v| v.as_str())
            .unwrap_or("?");

        let outcome_str = match outcome {
            "success" => outcome.green().to_string(),
            "denied" => outcome.yellow().to_string(),
            _ => outcome.red().to_string(),
        };

        println!(
            "  {} {:<8} {:<20} {:<30} {}",
            ts.bright_black(),
            outcome_str,
            actor,
            action,
            resource.bright_black()
        );
    }
    println!();
    output::dim(&format!("({} entries)", entries.len()));
    Ok(())
}
