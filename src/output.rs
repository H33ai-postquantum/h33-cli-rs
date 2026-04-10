//! Pretty terminal output helpers.

use colored::Colorize;

pub fn ok(msg: &str) {
    println!("{} {}", "✔".green(), msg);
}

pub fn info(msg: &str) {
    println!("{} {}", "→".blue(), msg);
}

pub fn warn(msg: &str) {
    eprintln!("{} {}", "!".yellow(), msg.yellow());
}

pub fn err(msg: &str) {
    eprintln!("{} {}", "✘".red(), msg.red());
}

pub fn dim(msg: &str) {
    println!("{}", msg.bright_black());
}

pub fn banner() {
    println!();
    println!("  {} {}", "H33".bold().blue(), "— post-quantum security in 2 minutes".bright_black());
    println!("  {}", "install.h33.ai · h33.ai · Patent pending".bright_black());
    println!();
}
