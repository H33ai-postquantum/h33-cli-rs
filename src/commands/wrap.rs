//! `h33 wrap <file>` — print the AI prompt the developer should paste into their terminal AI.

use crate::output;
use anyhow::Result;
use colored::Colorize;
use std::path::Path;

pub fn run(file: &str) -> Result<()> {
    if !Path::new(file).exists() {
        output::err(&format!("File not found: {}", file));
        std::process::exit(1);
    }
    println!();
    output::info(&format!("Substrate wrapping for {} is performed by your terminal AI.", file));
    println!();
    println!("{}", "Suggested prompt:".bold());
    println!();
    println!(
        "  {}",
        format!(
            "\"Wrap classical crypto in {} with H33 substrate. Read https://h33.ai/llms.txt \
             and https://h33.ai/detection-rules.yaml. Use the H33_AGENT_TOKEN already in my .env.\"",
            file
        )
        .bright_black()
    );
    println!();
    println!(
        "  {}",
        "Your AI agent will detect the patterns, apply substrate wrapping, and open a PR. \
         Time-to-PR target: under 3 minutes."
            .bright_black()
    );
    println!();
    Ok(())
}
