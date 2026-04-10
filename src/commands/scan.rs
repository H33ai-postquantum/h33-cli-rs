//! `h33 scan` — run a HICS cryptographic security scan.

use crate::{client::H33Client, config, output};
use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

const BASELINE_PATH: &str = ".h33/baseline.json";

pub async fn run(api_base: &str, path: &str, save_baseline: bool, diff: bool) -> Result<()> {
    let token = config::require_agent_token()?;
    output::banner();
    output::info(&format!("Running HICS scan on {}…", path));

    let baseline = if diff {
        Some(load_baseline().with_context(|| {
            format!(
                "no baseline found at {} — run 'h33 scan --baseline' first",
                BASELINE_PATH
            )
        })?)
    } else {
        None
    };

    let abs_path = PathBuf::from(path)
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(path));
    let body = json!({
        "path": abs_path.to_string_lossy(),
        "include_proof": true,
        "comparison_baseline": baseline,
    });

    let client = H33Client::new(api_base)?;
    let result = client.post_json("/v1/hics/scan", &token, body).await?;

    print_result(&result);

    if save_baseline {
        save_baseline_record(&result)?;
        output::ok(&format!("Baseline saved to {}", BASELINE_PATH));
        println!(
            "  {}",
            "After integrating H33, run 'h33 scan --diff' to see the improvement."
                .bright_black()
        );
        println!();
    }
    Ok(())
}

fn print_result(result: &Value) {
    let score = result.get("score").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let grade = result.get("grade").and_then(|v| v.as_str()).unwrap_or("?");
    let pq_ready = result
        .get("pq_ready")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let files = result
        .get("total_files")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let lines = result
        .get("total_lines")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let score_colored = if grade == "A+" || grade == "A" {
        format!("{} / 100 ({})", score as i64, grade).green().bold()
    } else if grade == "B" {
        format!("{} / 100 ({})", score as i64, grade).cyan().bold()
    } else if grade == "C" {
        format!("{} / 100 ({})", score as i64, grade).yellow().bold()
    } else {
        format!("{} / 100 ({})", score as i64, grade).red().bold()
    };

    println!();
    println!("  {} {}", "HICS Score:".bold(), score_colored);
    println!(
        "  {}    {}",
        "PQ Ready:".bold(),
        if pq_ready {
            "✓ yes".green().to_string()
        } else {
            "✗ no".red().to_string()
        }
    );
    println!("  {}       {}", "Files:".bold(), files);
    println!("  {}       {}", "Lines:".bold(), lines);
    println!();

    if let Some(comparison) = result.get("comparison") {
        let baseline_score = comparison
            .get("baseline_score")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let delta = comparison
            .get("delta")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let grade_change = comparison
            .get("grade_change")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let sign = if delta >= 0.0 { "+" } else { "" };
        let delta_colored = if delta >= 0.0 {
            format!("{}{}", sign, delta as i64).green().bold().to_string()
        } else {
            format!("{}{}", sign, delta as i64).red().bold().to_string()
        };
        println!("  {}    {} / 100", "Baseline:".bold(), baseline_score as i64);
        println!(
            "  {}       {}  {}",
            "Delta:".bold(),
            delta_colored,
            grade_change.bright_black()
        );
        println!();
    }

    if let Some(proof) = result.get("proof") {
        if let Some(id) = proof.get("stark_proof_id").and_then(|v| v.as_str()) {
            println!("  {}       {}", "Proof:".bold(), id.bright_black());
        }
        if let Some(url) = proof.get("verification_url").and_then(|v| v.as_str()) {
            println!("  {}      {}", "Verify:".bold(), url);
        }
    }
    println!();
}

fn save_baseline_record(result: &Value) -> Result<()> {
    let dir = Path::new(".h33");
    if !dir.exists() {
        fs::create_dir_all(dir)?;
    }
    let record = json!({
        "saved_at": chrono::Utc::now().to_rfc3339(),
        "scanned_path": result.get("scanned_path").cloned().unwrap_or(Value::Null),
        "score": result.get("score").cloned().unwrap_or(Value::Null),
        "grade": result.get("grade").cloned().unwrap_or(Value::Null),
        "proof_id": result.pointer("/proof/stark_proof_id").cloned().unwrap_or(Value::Null),
        "anchor": result.pointer("/proof/substrate_anchor").cloned().unwrap_or(Value::Null),
    });
    fs::write(BASELINE_PATH, serde_json::to_string_pretty(&record)?)?;
    Ok(())
}

fn load_baseline() -> Result<Value> {
    let content = fs::read_to_string(BASELINE_PATH)
        .with_context(|| format!("reading {}", BASELINE_PATH))?;
    let v: Value = serde_json::from_str(&content)?;
    Ok(v)
}
