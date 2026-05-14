//! `h33 prove` — compute a SHA3-256 fingerprint of the project and attest it
//! via the H33 Substrate, producing an H33-74 attestation receipt.

use crate::{client::H33Client, config, output};
use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::{json, Value};
use sha3::{Digest, Sha3_256};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;
use walkdir::WalkDir;

const ATTESTATION_PATH: &str = ".h33/latest-attestation.json";

pub async fn run(api_base: &str, path: &str) -> Result<()> {
    let token = config::api_key()
        .or_else(config::agent_token)
        .ok_or_else(|| {
            anyhow::anyhow!("No API key found. Run 'h33 init' to set up your account.")
        })?;

    output::banner();
    output::info(&format!("Computing SHA3-256 fingerprint of {}…", path));

    let start = Instant::now();
    let (fingerprint, file_count) = compute_fingerprint(path)?;
    let hash_elapsed = start.elapsed();

    output::ok(&format!(
        "Fingerprint: {} ({} files in {:.0}ms)",
        &fingerprint[..16].bright_black(),
        file_count,
        hash_elapsed.as_secs_f64() * 1000.0
    ));

    output::info("Attesting via H33 Substrate…");

    let body = json!({
        "data": fingerprint,
        "type": "BiometricAuth",
    });

    let client = H33Client::new(api_base)?;
    let result = client
        .post_json("/v1/substrate/attest", &token, body)
        .await
        .context("substrate attestation failed")?;

    print_attestation(&result, &fingerprint);
    save_receipt(&result, &fingerprint, path, file_count)?;

    Ok(())
}

/// Walk the project directory, hash all source files (.rs, .ts, .js, .py),
/// and return the hex-encoded SHA3-256 digest plus a file count.
fn compute_fingerprint(path: &str) -> Result<(String, usize)> {
    let source_exts: &[&str] = &["rs", "ts", "js", "py"];
    let skip_dirs: &[&str] = &[
        "node_modules",
        ".git",
        "dist",
        "build",
        ".next",
        "target",
        ".venv",
        "venv",
    ];

    let mut hasher = Sha3_256::new();
    let mut file_count: usize = 0;

    // Collect and sort paths for deterministic hashing
    let mut paths: Vec<PathBuf> = Vec::new();
    for entry in WalkDir::new(path)
        .sort_by_file_name()
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !skip_dirs.contains(&name.as_ref())
        })
    {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_file() {
            continue;
        }
        let Some(ext) = entry.path().extension().and_then(|s| s.to_str()) else {
            continue;
        };
        if !source_exts.contains(&ext) {
            continue;
        }
        paths.push(entry.path().to_path_buf());
    }

    for p in &paths {
        let Ok(content) = fs::read(p) else {
            continue;
        };
        // Include the relative path in the hash so renames change the fingerprint
        let rel = p
            .strip_prefix(path)
            .unwrap_or(p)
            .to_string_lossy();
        hasher.update(rel.as_bytes());
        hasher.update(&content);
        file_count += 1;
    }

    let digest = hasher.finalize();
    Ok((hex::encode(digest), file_count))
}

fn print_attestation(result: &Value, fingerprint: &str) {
    println!();
    println!(
        "  {} {}",
        "H33-74 Attestation".bold().green(),
        "— 74 bytes. Post-quantum attested. Forever.".bright_black()
    );
    println!();

    // Fingerprint
    println!("  {}  {}", "Fingerprint:".bold(), fingerprint.bright_black());

    // On-chain hash
    if let Some(hash) = result
        .get("on_chain_hash")
        .or_else(|| result.get("onchain_hash"))
        .or_else(|| result.get("hash"))
        .and_then(|v| v.as_str())
    {
        println!("  {}   {}", "On-chain:".bold(), hash.cyan());
    }

    // Receipt hex
    if let Some(receipt) = result
        .get("receipt")
        .or_else(|| result.get("receipt_hex"))
        .or_else(|| result.get("anchor"))
        .and_then(|v| v.as_str())
    {
        println!("  {}     {}", "Receipt:".bold(), receipt);
    }

    // Signature timings
    println!();
    if let Some(sigs) = result.get("signatures").or_else(|| result.get("timings")) {
        let ml_dsa = sigs
            .get("ml_dsa")
            .or_else(|| sigs.get("dilithium"))
            .and_then(|v| v.as_f64());
        let falcon = sigs.get("falcon").and_then(|v| v.as_f64());
        let sphincs = sigs
            .get("sphincs")
            .or_else(|| sigs.get("slh_dsa"))
            .and_then(|v| v.as_f64());

        if let Some(t) = ml_dsa {
            println!(
                "  {}    {:.2}ms",
                "ML-DSA-65:".bold(),
                t
            );
        }
        if let Some(t) = falcon {
            println!(
                "  {}  {:.2}ms",
                "FALCON-512:".bold(),
                t
            );
        }
        if let Some(t) = sphincs {
            println!(
                "  {}  {:.2}ms",
                "SPHINCS+:  ".bold(),
                t
            );
        }
    }

    // Compression ratio
    if let Some(ratio) = result
        .get("compression_ratio")
        .or_else(|| result.get("ratio"))
        .and_then(|v| v.as_f64())
    {
        println!(
            "  {} {:.0}x",
            "Compression:".bold(),
            ratio
        );
    }

    // Total latency
    if let Some(latency) = result
        .get("total_latency_ms")
        .or_else(|| result.get("latency_ms"))
        .or_else(|| result.get("elapsed_ms"))
        .and_then(|v| v.as_f64())
    {
        println!(
            "  {}     {:.1}ms",
            "Latency:".bold(),
            latency
        );
    }

    // Bytes
    if let Some(bytes) = result
        .get("total_bytes")
        .or_else(|| result.get("size_bytes"))
        .and_then(|v| v.as_u64())
    {
        println!("  {}        {} bytes", "Size:".bold(), bytes);
    }

    println!();
}

fn save_receipt(result: &Value, fingerprint: &str, path: &str, file_count: usize) -> Result<()> {
    let dir = Path::new(".h33");
    if !dir.exists() {
        fs::create_dir_all(dir)?;
    }
    let record = json!({
        "saved_at": chrono::Utc::now().to_rfc3339(),
        "path": path,
        "file_count": file_count,
        "fingerprint": fingerprint,
        "attestation": result,
    });
    fs::write(ATTESTATION_PATH, serde_json::to_string_pretty(&record)?)?;
    output::ok(&format!("Receipt saved to {}", ATTESTATION_PATH));
    println!(
        "  {}",
        "Verify anytime: h33 verify <on-chain-hash>".bright_black()
    );
    println!();
    Ok(())
}
