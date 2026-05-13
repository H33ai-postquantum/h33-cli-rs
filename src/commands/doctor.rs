//! `h33 doctor` — verify environment, connectivity, auth, and config.
//!
//! Checks:
//! 1. Config readable (~/.h33/config.toml exists and parses)
//! 2. Auth base reachable (TLS + HTTP)
//! 3. API base reachable (TLS + HTTP)
//! 4. API key valid (authenticated request succeeds)
//! 5. TLS valid (certificate chain OK)
//! 6. Latency (round-trip to both endpoints)
//! 7. Quota remaining

use crate::{client::join_url, config, output};
use anyhow::Result;
use colored::Colorize;
use std::time::Instant;

struct Check {
    name: &'static str,
    passed: bool,
    detail: String,
}

impl Check {
    fn pass(name: &'static str, detail: impl Into<String>) -> Self {
        Self { name, passed: true, detail: detail.into() }
    }
    fn fail(name: &'static str, detail: impl Into<String>) -> Self {
        Self { name, passed: false, detail: detail.into() }
    }
}

pub async fn run(api_base: &str, auth_base: &str) -> Result<()> {
    output::banner();
    println!();
    println!("  {}", "h33 doctor".bold().underline());
    println!("  Checking environment, connectivity, and auth...");
    println!();

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent(concat!("h33-cli/", env!("CARGO_PKG_VERSION")))
        .build()?;

    let mut checks: Vec<Check> = Vec::new();

    // ── 1. Config readable ──
    let config_check = match config::api_key() {
        Some(key) => {
            // Only show last 4 chars — developers paste screenshots
            let suffix = if key.len() >= 4 { &key[key.len()-4..] } else { &key };
            let env_label = if key.starts_with("h33_sand") || key.starts_with("h33_offline") {
                "Sandbox"
            } else if key.starts_with("h33_live") {
                "Live"
            } else if key.starts_with("h33_test") {
                "Test"
            } else {
                "Key"
            };
            Check::pass("Config", format!("{env_label} key configured (…{suffix})"))
        }
        None => Check::fail("Config", "No API key found. Run `h33 init` or set H33_API_KEY."),
    };
    checks.push(config_check);

    // ── 2. Auth base reachable ──
    let auth_check = check_endpoint(&client, auth_base, "/api/status").await;
    checks.push(auth_check);

    // ── 3. API base reachable ──
    let api_check = check_endpoint(&client, api_base, "/health").await;
    checks.push(api_check);

    // ── 4. TLS valid ──
    // If endpoints responded over HTTPS without error, TLS is valid.
    // The reqwest client validates certs by default (rustls).
    let auth_tls = checks.get(1).map(|c| c.passed).unwrap_or(false);
    let api_tls = checks.get(2).map(|c| c.passed).unwrap_or(false);
    if auth_tls && api_tls {
        checks.push(Check::pass("TLS", "Certificate chains valid (rustls)"));
    } else if auth_tls || api_tls {
        checks.push(Check::fail("TLS", "One endpoint has TLS issues"));
    } else {
        checks.push(Check::fail("TLS", "Cannot verify — both endpoints unreachable"));
    }

    // ── 5. Key valid + quota ──
    if let Some(api_key) = config::api_key() {
        let key_check = check_key_and_quota(&client, api_base, &api_key).await;
        checks.extend(key_check);
    } else {
        checks.push(Check::fail("Key Valid", "No key to validate"));
        checks.push(Check::fail("Quota", "Cannot check — no key"));
    }

    // ── Print results ──
    let total = checks.len();
    let passed = checks.iter().filter(|c| c.passed).count();
    let failed = total - passed;

    for check in &checks {
        if check.passed {
            println!(
                "  {} {} — {}",
                "✔".green().bold(),
                check.name.bold(),
                check.detail.bright_black()
            );
        } else {
            println!(
                "  {} {} — {}",
                "✗".red().bold(),
                check.name.bold(),
                check.detail.red()
            );
        }
    }

    println!();
    if failed == 0 {
        println!(
            "  {} All {} checks passed.",
            "✔".green().bold(),
            total
        );
    } else {
        println!(
            "  {} {}/{} checks passed, {} failed.",
            "⚠".yellow().bold(),
            passed,
            total,
            failed
        );
    }

    println!();
    println!("  {}  {}", "Auth Base:".bold(), auth_base.bright_black());
    println!("  {}   {}", "API Base:".bold(), api_base.bright_black());
    println!();

    if failed > 0 {
        std::process::exit(1);
    }
    Ok(())
}

async fn check_endpoint(client: &reqwest::Client, base: &str, path: &str) -> Check {
    let url = join_url(base, path);
    let label = if path.contains("status") { "Auth Reachable" } else { "API Reachable" };

    let start = Instant::now();
    match client.get(&url).send().await {
        Ok(resp) => {
            let latency = start.elapsed();
            let status = resp.status().as_u16();
            // Any HTTP response (even 401/403) means the endpoint is reachable
            if status < 500 {
                Check::pass(
                    label,
                    format!("{base} — HTTP {status}, {:.0}ms", latency.as_secs_f64() * 1000.0),
                )
            } else {
                Check::fail(label, format!("{base} — HTTP {status} (server error)"))
            }
        }
        Err(e) => {
            let msg = if e.is_timeout() {
                format!("{base} — connection timed out")
            } else if e.is_connect() {
                format!("{base} — connection refused")
            } else {
                format!("{base} — {e}")
            };
            Check::fail(label, msg)
        }
    }
}

async fn check_key_and_quota(
    client: &reqwest::Client,
    api_base: &str,
    api_key: &str,
) -> Vec<Check> {
    let mut out = Vec::new();
    let url = join_url(api_base, "/v1/tenant/quota");

    let start = Instant::now();
    let result = client
        .get(&url)
        .header("X-API-Key", api_key)
        .send()
        .await;

    match result {
        Ok(resp) => {
            let latency = start.elapsed();
            let status = resp.status().as_u16();
            if status == 200 {
                match resp.json::<serde_json::Value>().await {
                    Ok(data) => {
                        out.push(Check::pass(
                            "Key Valid",
                            format!("Authenticated — {:.0}ms", latency.as_secs_f64() * 1000.0),
                        ));
                        let remaining = data.get("remaining").and_then(|v| v.as_u64()).unwrap_or(0);
                        let quota = data.get("quota").and_then(|v| v.as_u64()).unwrap_or(0);
                        let pct = if quota > 0 { (remaining as f64 / quota as f64) * 100.0 } else { 0.0 };
                        if remaining == 0 && quota > 0 {
                            out.push(Check::fail("Quota", format!("0 / {} — exhausted", quota)));
                        } else {
                            out.push(Check::pass(
                                "Quota",
                                format!("{} / {} remaining ({:.0}%)", remaining, quota, pct),
                            ));
                        }
                    }
                    Err(_) => {
                        out.push(Check::pass("Key Valid", format!("HTTP 200 — {:.0}ms", latency.as_secs_f64() * 1000.0)));
                        out.push(Check::fail("Quota", "Could not parse quota response"));
                    }
                }
            } else if status == 401 || status == 403 {
                out.push(Check::fail("Key Valid", format!("HTTP {status} — key rejected")));
                out.push(Check::fail("Quota", "Cannot check — key invalid"));
            } else {
                out.push(Check::fail("Key Valid", format!("HTTP {status}")));
                out.push(Check::fail("Quota", "Cannot check"));
            }
        }
        Err(e) => {
            out.push(Check::fail("Key Valid", format!("Request failed: {e}")));
            out.push(Check::fail("Quota", "Cannot check — API unreachable"));
        }
    }

    // ── Latency check ──
    let url = join_url(api_base, "/health");
    let start = Instant::now();
    if let Ok(resp) = client.get(&url).send().await {
        let ms = start.elapsed().as_secs_f64() * 1000.0;
        let _ = resp; // consume
        if ms < 200.0 {
            out.push(Check::pass("Latency", format!("{:.0}ms (excellent)", ms)));
        } else if ms < 1000.0 {
            out.push(Check::pass("Latency", format!("{:.0}ms (acceptable)", ms)));
        } else {
            out.push(Check::fail("Latency", format!("{:.0}ms (high — check network)", ms)));
        }
    } else {
        out.push(Check::fail("Latency", "Could not measure — API unreachable"));
    }

    out
}
