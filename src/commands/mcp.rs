//! `h33 mcp` — spawn the H33 MCP server subprocess.

use crate::{config, output};
use anyhow::{Context, Result};

pub async fn run(api_base: &str) -> Result<()> {
    let token = config::require_agent_token()?;
    if !token.starts_with("cka_") {
        output::err(
            "H33_AGENT_TOKEN must be a cka_* agent capability token. Use 'h33 mint' to create one.",
        );
        std::process::exit(1);
    }

    output::info("Starting H33 MCP server (stdio)…");

    let mcp_bin = std::env::var("H33_MCP_BIN").unwrap_or_else(|_| "h33-mcp".to_string());
    let mut child = tokio::process::Command::new(&mcp_bin)
        .env("H33_AGENT_TOKEN", &token)
        .env("H33_API_BASE", api_base)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .spawn()
        .with_context(|| format!("spawning {}", mcp_bin))?;

    let status = child.wait().await?;
    std::process::exit(status.code().unwrap_or(0));
}
