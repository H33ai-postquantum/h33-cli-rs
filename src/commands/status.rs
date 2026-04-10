//! `h33 status` — print tenant metadata and quota.

use crate::{client::H33Client, config, output};
use anyhow::Result;
use colored::Colorize;

pub async fn run(api_base: &str) -> Result<()> {
    let api_key = config::require_api_key()?;
    output::banner();

    let client = H33Client::new(api_base)?;
    let tenant = client.get_json("/v1/tenant", Some(&api_key)).await?;
    let quota = client.get_json("/v1/tenant/quota", Some(&api_key)).await?;

    let name = tenant.get("name").and_then(|v| v.as_str()).unwrap_or("?");
    let id = tenant.get("id").and_then(|v| v.as_str()).unwrap_or("?");
    let tier = tenant.get("tier").and_then(|v| v.as_str()).unwrap_or("?");
    let status = tenant
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("?");

    println!();
    println!("  {} {} ({})", "Tenant:".bold(), name, id);
    println!("  {}   {}", "Tier:".bold(), tier);
    println!(
        "  {} {}",
        "Status:".bold(),
        if status == "active" {
            status.green().to_string()
        } else {
            status.yellow().to_string()
        }
    );
    println!();

    let used = quota.get("used").and_then(|v| v.as_u64()).unwrap_or(0);
    let limit = quota.get("quota").and_then(|v| v.as_u64()).unwrap_or(0);
    let remaining = quota.get("remaining").and_then(|v| v.as_u64()).unwrap_or(0);
    let resets_at = quota
        .get("resets_at")
        .and_then(|v| v.as_str())
        .unwrap_or("?");

    println!(
        "  {}      {} / {}",
        "Quota:".bold(),
        format_number(used),
        format_number(limit)
    );
    println!("  {}  {}", "Remaining:".bold(), format_number(remaining));
    println!("  {}     {}", "Resets:".bold(), resets_at);
    println!();
    Ok(())
}

fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut chars: Vec<char> = s.chars().rev().collect();
    let mut out = String::new();
    for (i, c) in chars.iter_mut().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(*c);
    }
    out.chars().rev().collect()
}
