//! `h33 signup` — open the H33 signup page in the default browser.

use crate::output;
use anyhow::Result;
use colored::Colorize;

const SIGNUP_URL: &str = "https://h33.ai/signup";

pub async fn run() -> Result<()> {
    output::banner();
    output::info("Opening signup page…");
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(target_os = "windows") {
        "start"
    } else {
        "xdg-open"
    };
    let _ = tokio::process::Command::new(opener)
        .arg(SIGNUP_URL)
        .spawn();

    println!();
    println!("  {} {}", "Signup URL:".bold(), SIGNUP_URL);
    println!();
    println!("  {}", "Next:".bold());
    println!("    1. Sign up (free tier, no credit card required)");
    println!("    2. Copy your {} key from the dashboard", "ck_test_*".bold());
    println!("    3. Add it to .env: {}", "H33_API_KEY=ck_test_...".bright_black());
    println!("    4. Run {} to create an agent token", "h33 mint".bold());
    println!();
    Ok(())
}
