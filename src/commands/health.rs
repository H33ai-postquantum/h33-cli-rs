//! `h33 health` — check the H33 API health endpoint.

use crate::{client::H33Client, output};
use anyhow::Result;
use colored::Colorize;

pub async fn run(api_base: &str) -> Result<()> {
    let client = H33Client::new(api_base)?;
    match client.get_json("/health", None).await {
        Ok(data) => {
            let status = data
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let version = data
                .get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            output::ok(&format!(
                "API {}: {} {}",
                api_base,
                status.green(),
                if version.is_empty() {
                    String::new()
                } else {
                    format!("(v{})", version)
                }
            ));
            Ok(())
        }
        Err(e) => {
            output::err(&format!("{}: {}", api_base, e));
            std::process::exit(1);
        }
    }
}
