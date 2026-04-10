//! `h33 detect` — run detection-rules.yaml against the current repo.

use crate::output;
use anyhow::Result;
use colored::Colorize;
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

const RULES_URL: &str = "https://h33.ai/detection-rules.yaml";

#[derive(Debug, Clone)]
struct Rule {
    id: String,
    pattern: String,
    regex: Regex,
    domain: String,
    severity: String,
}

#[derive(Debug)]
struct Match {
    rule_id: String,
    severity: String,
    domain: String,
    file: PathBuf,
    line: usize,
    pattern: String,
}

pub async fn run(path: &str) -> Result<()> {
    output::banner();
    output::info("Fetching detection rules from h33.ai…");

    let rules_text = reqwest::Client::new()
        .get(RULES_URL)
        .header("User-Agent", concat!("h33-cli/", env!("CARGO_PKG_VERSION")))
        .send()
        .await?
        .text()
        .await?;

    // Minimal YAML parser — extract rule IDs, patterns, domains, and severity
    let rules = parse_rules(&rules_text);
    output::ok(&format!("Loaded {} detection rules", rules.len()));
    output::info(&format!("Scanning {}…", path));

    let skip_dirs: &[&str] = &[
        "node_modules", ".git", "dist", "build", ".next", "target", ".venv", "venv",
    ];
    let ext_ok: &[&str] = &[
        "js", "ts", "jsx", "tsx", "py", "go", "rs", "rb", "java", "kt", "cs", "php", "sh",
        "c", "cpp", "h", "hpp", "sol",
    ];

    let mut matches: Vec<Match> = Vec::new();
    for entry in WalkDir::new(path).into_iter().filter_entry(|e| {
        let name = e.file_name().to_string_lossy();
        !skip_dirs.contains(&name.as_ref())
    }) {
        let Ok(entry) = entry else { continue };
        if !entry.file_type().is_file() {
            continue;
        }
        let Some(ext) = entry.path().extension().and_then(|s| s.to_str()) else {
            continue;
        };
        if !ext_ok.contains(&ext) {
            continue;
        }
        if entry.metadata().map(|m| m.len()).unwrap_or(0) > 1_048_576 {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        for (line_num, line) in content.lines().enumerate() {
            for rule in &rules {
                if rule.regex.is_match(line) {
                    matches.push(Match {
                        rule_id: rule.id.clone(),
                        severity: rule.severity.clone(),
                        domain: rule.domain.clone(),
                        file: entry.path().to_path_buf(),
                        line: line_num + 1,
                        pattern: rule.pattern.clone(),
                    });
                }
            }
        }
    }

    println!();
    if matches.is_empty() {
        output::ok("No classical crypto patterns detected.");
        println!();
        return Ok(());
    }

    let unique_files: std::collections::HashSet<_> =
        matches.iter().map(|m| m.file.clone()).collect();
    println!(
        "{}",
        format!(
            "Found {} matches across {} files:",
            matches.len(),
            unique_files.len()
        )
        .bold()
    );
    println!();

    // Group by file
    let mut by_file: HashMap<PathBuf, Vec<&Match>> = HashMap::new();
    for m in &matches {
        by_file.entry(m.file.clone()).or_default().push(m);
    }
    let base = Path::new(path).canonicalize().unwrap_or_else(|_| path.into());
    for (file, file_matches) in &by_file {
        let display = file
            .strip_prefix(&base)
            .unwrap_or(file)
            .display()
            .to_string();
        println!("  {}", display.cyan());
        for m in file_matches {
            let sev = match m.severity.as_str() {
                "critical" => m.severity.red().to_string(),
                "high" => m.severity.yellow().to_string(),
                _ => m.severity.bright_black().to_string(),
            };
            println!(
                "    {} {:<8} {:<10} {} {}",
                format!("L{}", m.line).bright_black(),
                sev,
                m.rule_id,
                m.domain.cyan(),
                m.pattern.bright_black()
            );
        }
    }
    println!();
    println!("{}", "Next:".bold());
    println!(
        "  Run {} to have your AI agent apply substrate wrapping.",
        "h33 wrap <file>".bold()
    );
    println!();
    Ok(())
}

/// Minimal rule parser — extracts `- id:`, `pattern:`, `substrate_domain_id:`,
/// and `severity:` lines. Not a full YAML parser but sufficient for the
/// detection rules schema.
fn parse_rules(text: &str) -> Vec<Rule> {
    let mut out: Vec<Rule> = Vec::new();
    let mut cur_id: Option<String> = None;
    let mut cur_pattern: Option<String> = None;
    let mut cur_domain = String::new();
    let mut cur_severity = String::new();

    let id_re = Regex::new(r"^-\s*id:\s*(\S+)").ok();
    let pat_re = Regex::new(r#"^\s+pattern:\s*['"]?(.+?)['"]?\s*$"#).ok();
    let dom_re = Regex::new(r#"^\s+substrate_domain_id:\s*['"]?(\S+?)['"]?\s*$"#).ok();
    let sev_re = Regex::new(r"^\s+severity:\s*(\S+)").ok();

    for line in text.lines() {
        if let Some(re) = &id_re {
            if let Some(c) = re.captures(line) {
                // Flush previous rule
                if let (Some(id), Some(pat)) = (cur_id.take(), cur_pattern.take()) {
                    if let Ok(r) = Regex::new(&pat) {
                        out.push(Rule {
                            id,
                            pattern: pat,
                            regex: r,
                            domain: std::mem::take(&mut cur_domain),
                            severity: std::mem::take(&mut cur_severity),
                        });
                    }
                }
                cur_id = Some(c[1].to_string());
                continue;
            }
        }
        if let Some(re) = &pat_re {
            if let Some(c) = re.captures(line) {
                cur_pattern = Some(c[1].to_string());
                continue;
            }
        }
        if let Some(re) = &dom_re {
            if let Some(c) = re.captures(line) {
                cur_domain = c[1].to_string();
                continue;
            }
        }
        if let Some(re) = &sev_re {
            if let Some(c) = re.captures(line) {
                cur_severity = c[1].to_string();
                continue;
            }
        }
    }
    // Flush final rule
    if let (Some(id), Some(pat)) = (cur_id, cur_pattern) {
        if let Ok(r) = Regex::new(&pat) {
            out.push(Rule {
                id,
                pattern: pat,
                regex: r,
                domain: cur_domain,
                severity: cur_severity,
            });
        }
    }
    out
}
