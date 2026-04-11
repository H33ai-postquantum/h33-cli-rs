//! `h33 verify <target>` — verify a substrate anchor by ID OR verify an
//! HTTP response by URL.
//!
//! This command has two dispatches behind a single public entry point:
//!
//!   1. **Target looks like a URL** (`http://` or `https://` prefix) →
//!      fetches the response, parses the four `X-H33-*` attestation
//!      headers via `h33-substrate-verifier`, runs the four structural
//!      integrity checks locally (no network call beyond the initial
//!      GET), and prints a colored per-check verdict.
//!
//!   2. **Target looks like an anchor ID** (anything else) → posts to
//!      the legacy `/v1/substrate/verify` endpoint with the anchor_id
//!      as the JSON body and prints the server's verdict.
//!
//! The dual dispatch is what makes the HICS badge clickable: a
//! customer who sees "H33 Verified" on a GitHub PR page can open a
//! terminal and type `h33 verify <url>`, getting the full offline
//! verification result in under 2 seconds.

use crate::{client::H33Client, config, output};
use anyhow::{anyhow, Result};
use colored::Colorize;
use h33_substrate_verifier::{
    headers::headers_from_reqwest, Verifier, VerificationResult,
};
use serde_json::json;
use std::time::Instant;

/// Entry point. Decides which dispatch to use based on whether the
/// caller gave us a URL or an anchor ID.
pub async fn run(api_base: &str, target: &str) -> Result<()> {
    if is_url(target) {
        verify_url(target).await
    } else {
        verify_anchor(api_base, target).await
    }
}

/// A target is a URL if it starts with `http://` or `https://`. We do
/// not accept scheme-less or file:// targets — if a customer wants to
/// verify a saved response, they can pipe it through in a future
/// iteration.
fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

// ─── URL verification path ───────────────────────────────────────────

async fn verify_url(url: &str) -> Result<()> {
    output::banner();
    println!();
    println!(
        "  {}  {}",
        "Verifying:".bold(),
        url.bright_cyan()
    );
    println!();

    let t0 = Instant::now();

    // Use reqwest directly so we have control over the response object
    // (the h33-substrate-verifier::headers_from_reqwest helper takes a
    // &reqwest::Response, so we need to hold on to it).
    let client = reqwest::Client::builder()
        .user_agent(concat!("h33-cli/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|e| anyhow!("reqwest client build failed: {e}"))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| anyhow!("request failed: {e}"))?;

    let status = response.status();
    let fetch_ms = t0.elapsed().as_millis();

    println!(
        "  {}      HTTP {} ({} ms)",
        "Status:".bold(),
        if status.is_success() {
            status.to_string().bright_green()
        } else {
            status.to_string().bright_yellow()
        },
        fetch_ms
    );

    // Attestation headers. If any are missing, the endpoint is either
    // in the scif-backend skip list (/health, /ready, /metrics,
    // /v1/auth/speedtest/*) or the server has not yet shipped the
    // substrate response middleware.
    let owned_headers = match headers_from_reqwest(&response) {
        Ok(h) => h,
        Err(e) => {
            println!();
            output::err(
                "This response does not carry H33 attestation headers.",
            );
            println!();
            println!("  {}", format!("{e}").bright_black());
            println!();
            println!(
                "  {}",
                "Likely causes:".bold()
            );
            println!(
                "    · The endpoint is in the server's attestation skip list"
            );
            println!(
                "      (/health, /ready, /metrics, /v1/auth/speedtest/*)"
            );
            println!("    · The server runs an older scif-backend without the");
            println!("      substrate response middleware");
            println!(
                "    · A reverse proxy in front of the server stripped the"
            );
            println!(
                "      X-H33-* headers (check your CORS / proxy allowlist)"
            );
            println!();
            println!(
                "  Try an endpoint that is guaranteed to be attested, e.g."
            );
            println!(
                "    {}",
                "h33 verify https://api.h33.ai/v1/substrate/public-keys"
                    .bright_cyan()
            );
            println!();
            return Err(anyhow!("missing X-H33-* attestation headers"));
        }
    };

    // Buffer the response body for SHA3 hashing.
    let body = response
        .bytes()
        .await
        .map_err(|e| anyhow!("response body read failed: {e}"))?;

    let body_size = body.len();
    let verify_start = Instant::now();

    let verifier = Verifier::new();
    let result = verifier
        .verify(&body, &owned_headers.borrow())
        .map_err(|e| anyhow!("verifier error: {e}"))?;

    let verify_us = verify_start.elapsed().as_micros();

    print_result_tree(&result, body_size, verify_us);

    if result.is_valid() {
        output::ok("H33 substrate response attestation verified");
        println!();
        Ok(())
    } else {
        output::err(&format!(
            "Verification failed: {}",
            result.summary()
        ));
        println!();
        // Non-zero exit so shell scripts can detect the failure.
        std::process::exit(1);
    }
}

fn print_result_tree(
    result: &VerificationResult,
    body_size: usize,
    verify_us: u128,
) {
    println!();
    println!("  {}", "Attestation headers".bold());
    println!(
        "    {} {}  body SHA3-256",
        check_mark(result.body_hash_matches),
        "X-H33-Substrate".dimmed(),
    );
    println!(
        "    {} {}     42-byte CompactReceipt",
        check_mark(result.receipt_well_formed),
        "X-H33-Receipt".dimmed(),
    );
    println!(
        "    {} {}  Dilithium + FALCON + SPHINCS+",
        check_mark(result.algorithms_match_flags),
        "X-H33-Algorithms".dimmed(),
    );
    println!(
        "    {} {}  substrate timestamp",
        check_mark(result.timestamps_agree),
        "X-H33-Substrate-Ts".dimmed(),
    );
    println!();
    println!("  {}", "Computed".bold());
    println!(
        "    body hash:     {}",
        hex_short(&result.computed_body_hash).bright_black()
    );
    println!(
        "    body size:     {} bytes",
        body_size.to_string().bright_black()
    );
    if let Some(flags) = result.flags_from_receipt {
        println!(
            "    sig families:  {}{}{}",
            if flags.has_dilithium() {
                "Dilithium ".green()
            } else {
                "Dilithium ".dimmed()
            },
            if flags.has_falcon() {
                "FALCON ".green()
            } else {
                "FALCON ".dimmed()
            },
            if flags.has_sphincs() {
                "SPHINCS+".green()
            } else {
                "SPHINCS+".dimmed()
            },
        );
    }
    println!(
        "    verify time:   {} µs",
        verify_us.to_string().bright_black()
    );
    println!();
}

fn check_mark(ok: bool) -> colored::ColoredString {
    if ok {
        "✓".bright_green()
    } else {
        "✗".bright_red()
    }
}

fn hex_short(bytes: &[u8]) -> String {
    let full = hex::encode(bytes);
    if full.len() <= 16 {
        full
    } else {
        format!("{}…{}", &full[..8], &full[full.len() - 8..])
    }
}

// ─── Legacy anchor_id verification path (unchanged semantics) ────────

async fn verify_anchor(api_base: &str, anchor_id: &str) -> Result<()> {
    let token = config::require_agent_token()?;
    let client = H33Client::new(api_base)?;
    let body = json!({ "anchor_id": anchor_id });
    let result = client
        .post_json("/v1/substrate/verify", &token, body)
        .await?;

    println!();
    if result
        .get("valid")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        let domain = result
            .get("domain")
            .and_then(|v| v.as_str())
            .unwrap_or("?");
        let age = result
            .get("age_ms")
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        output::ok(&format!(
            "Anchor verified · domain={domain} · age={age}ms"
        ));
    } else {
        output::err("Anchor invalid");
    }
    if let Some(created_at) = result.get("created_at").and_then(|v| v.as_str()) {
        println!("  {} {}", "Created:".bold(), created_at);
    }
    println!();
    Ok(())
}
