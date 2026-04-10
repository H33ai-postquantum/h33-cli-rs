//! `h33 bitcoin` — Bitcoin UTXO quantum insurance commands.

use crate::{client::H33Client, config, output};
use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::{json, Value};
use std::fs;

pub async fn attest(api_base: &str, utxo: &str, address: &str, proof_path: &str) -> Result<()> {
    let token = config::require_agent_token()?;
    output::banner();
    output::info(&format!("Attesting UTXO {} (owner: {})…", utxo, address));

    let proof_json = fs::read_to_string(proof_path)
        .with_context(|| format!("reading proof file {}", proof_path))?;
    let ownership_proof: Value = serde_json::from_str(&proof_json)
        .with_context(|| format!("parsing proof JSON from {}", proof_path))?;

    let body = json!({
        "utxo": utxo,
        "owner_address": address,
        "ownership_proof": ownership_proof,
        "storage_tier": "arweave_permanent",
    });

    let client = H33Client::new(api_base)?;
    let result = client.post_json("/v1/bitcoin/attest", &token, body).await?;

    let attestation_id = result
        .get("attestation_id")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let arweave = result
        .get("arweave_tx_id")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let commitment = result
        .get("on_chain_commitment")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let anchor = result
        .get("substrate_anchor")
        .and_then(|v| v.as_str())
        .unwrap_or("?");
    let verify_url = result
        .get("verification_url")
        .and_then(|v| v.as_str())
        .unwrap_or("https://h33.ai/verify");

    output::ok(&format!("Attestation created: {}", attestation_id.green()));
    println!();
    println!("  {}   {}", "UTXO:".bold(), utxo);
    println!("  {} {}", "Owner:".bold(), address);
    println!(
        "  {}   {} {}",
        "On-chain commitment:".bold(),
        commitment.chars().take(32).collect::<String>().cyan(),
        "...".bright_black()
    );
    println!("  {}         {}", "Arweave TX:".bold(), arweave.bright_black());
    println!(
        "  {}    {} bytes (ready for Bitcoin OP_RETURN)",
        "Substrate anchor:".bold(),
        anchor.len() / 2
    );
    println!("  {}         {}", "Verify URL:".bold(), verify_url);
    println!();
    output::info("The full three-family signature bundle is permanently on Arweave.");
    output::info("Run 'h33 bitcoin verify <attestation_id>' to verify at any time. No account required.");
    println!();

    // Save a local copy of the attestation bundle
    let bundle_path = format!(".h33/bitcoin-{}.json", attestation_id);
    if let Err(e) = fs::create_dir_all(".h33") {
        output::warn(&format!("couldn't create .h33 dir: {}", e));
    } else if let Err(e) = fs::write(&bundle_path, serde_json::to_string_pretty(&result)?) {
        output::warn(&format!("couldn't save bundle copy: {}", e));
    } else {
        output::ok(&format!("Attestation bundle saved to {}", bundle_path));
    }
    println!();
    Ok(())
}

pub async fn verify(api_base: &str, attestation_id: &str) -> Result<()> {
    // Public endpoint — no auth required
    let client = H33Client::new(api_base)?;
    let result = client
        .get_json(&format!("/v1/bitcoin/verify/{}", attestation_id), None)
        .await?;

    let valid = result.get("valid").and_then(|v| v.as_bool()).unwrap_or(false);
    let utxo = result.get("utxo").and_then(|v| v.as_str()).unwrap_or("?");

    println!();
    if valid {
        output::ok(&format!("Attestation {} VALID", attestation_id));
    } else {
        output::err(&format!("Attestation {} INVALID", attestation_id));
    }
    println!("  {} {}", "UTXO:".bold(), utxo);

    if let Some(sigs) = result.get("signatures_verified") {
        println!();
        println!("  {}", "Signatures verified:".bold());
        let check = |name: &str, key: &str| {
            let ok = sigs.get(key).and_then(|v| v.as_bool()).unwrap_or(false);
            let mark = if ok { "✓".green() } else { "✗".red() };
            println!("    {}  {}", mark, name);
        };
        check("Dilithium (ML-DSA-65)", "dilithium");
        check("FALCON-512", "falcon");
        check("SPHINCS+ (SLH-DSA)", "sphincs_plus");
    }
    if let Some(arweave) = result.get("arweave_tx_id").and_then(|v| v.as_str()) {
        println!();
        println!("  {} {}", "Arweave TX:".bold(), arweave.bright_black());
    }
    println!();
    Ok(())
}

pub async fn lookup(api_base: &str, utxo: &str) -> Result<()> {
    // Public endpoint — no auth required
    let client = H33Client::new(api_base)?;
    let path = format!("/v1/bitcoin/lookup?utxo={}", urlencode(utxo));
    let result = client.get_json(&path, None).await?;

    let count = result
        .get("attestation_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    println!();
    if count == 0 {
        output::info(&format!("No attestations found for UTXO {}", utxo));
    } else {
        println!(
            "{}",
            format!("Found {} attestation(s) for UTXO {}:", count, utxo).bold()
        );
        println!();
        if let Some(attestations) = result.get("attestations").and_then(|v| v.as_array()) {
            for att in attestations {
                let id = att
                    .get("attestation_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let at = att
                    .get("attested_at")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                let arweave = att
                    .get("arweave_tx_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("?");
                println!("  {}", id.green());
                println!("    attested_at: {}", at.bright_black());
                println!("    arweave:     {}", arweave.bright_black());
                println!();
            }
        }
    }
    Ok(())
}

fn urlencode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '~' {
            out.push(c);
        } else {
            for b in c.to_string().as_bytes() {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}
