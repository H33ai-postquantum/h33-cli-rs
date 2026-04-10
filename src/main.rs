//! h33 — H33 Terminal Companion (native Rust)
//!
//! The canonical H33 CLI. Replaces the TypeScript version.
//! Zero Node dependencies. Single static binary. Signed release distribution.
//!
//! Commands:
//!   h33 signup          Open browser → sign up, paste API key
//!   h33 mint [opts]     Mint a cka_* agent capability token
//!   h33 mcp             Run the H33 MCP server (stdio subprocess)
//!   h33 status          Print quota, tier, and tenant info
//!   h33 audit [opts]    Show recent agent audit log entries
//!   h33 domains         List all substrate registry domain identifiers
//!   h33 detect          Run detection-rules.yaml against the current repo
//!   h33 wrap <file>     Print AI prompt to wrap classical crypto in a file
//!   h33 verify <anchor> Verify a substrate anchor by ID
//!   h33 scan [opts]     Run a HICS cryptographic security scan
//!   h33 bitcoin <cmd>   Bitcoin UTXO quantum insurance (attest/verify/lookup)
//!   h33 health          Check the H33 API health endpoint
//!   h33 version         Show CLI version
//!
//! The architectural rule: agents hold cka_*, servers hold ck_live_*,
//! they are never the same thing. This CLI never persists cka_* tokens.

mod client;
mod commands;
mod config;
mod output;

use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "h33",
    version,
    about = "H33 terminal companion — post-quantum security in 2 minutes",
    long_about = "The canonical H33 CLI. Sign up, mint agent tokens, run the MCP server, \
                  audit sessions, detect classical crypto, run HICS scans, attest Bitcoin UTXOs. \
                  Native Rust. Patent pending. SOC 2 Type II closes June 3, 2026."
)]
struct Cli {
    /// H33 API base URL (default: sandbox)
    #[arg(long, env = "H33_API_BASE", default_value = "https://sandbox.api.h33.ai", global = true)]
    api_base: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Open browser → sign up for H33, get your ck_test_* API key
    Signup,

    /// Mint a short-lived cka_* agent capability token
    Mint {
        /// Token lifetime in seconds (default 3600, max 86400)
        #[arg(long, default_value_t = 3600)]
        ttl: u64,
        /// Promote to production (requires dashboard approval)
        #[arg(long, default_value_t = false)]
        production: bool,
        /// Human user identifier for audit attribution
        #[arg(long, env = "USER")]
        user: String,
        /// Agent identifier (e.g., claude-code/2.x)
        #[arg(long, default_value = "h33-cli/0.1")]
        agent: String,
    },

    /// Run the H33 MCP server (stdio subprocess)
    Mcp,

    /// Print quota, tier, and tenant info
    Status,

    /// Show recent agent audit log entries
    Audit {
        /// Maximum entries to return
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },

    /// List all substrate registry domain identifiers
    Domains,

    /// Run detection-rules.yaml against the current repo
    Detect {
        /// Path to scan (default: current directory)
        path: Option<String>,
    },

    /// Print an AI prompt to wrap classical crypto in a specific file
    Wrap {
        /// File to wrap
        file: String,
    },

    /// Verify a substrate anchor by ID
    Verify {
        /// The 74-byte anchor ID
        anchor_id: String,
    },

    /// Run a HICS cryptographic security scan
    Scan {
        /// Path to scan (default: current directory)
        path: Option<String>,
        /// Save the result as the baseline for future --diff comparisons
        #[arg(long)]
        baseline: bool,
        /// Compare against the saved baseline (.h33/baseline.json)
        #[arg(long)]
        diff: bool,
    },

    /// Bitcoin UTXO quantum insurance (attest/verify/lookup)
    Bitcoin {
        #[command(subcommand)]
        sub: BitcoinCommand,
    },

    /// Check the H33 API health endpoint
    Health,
}

#[derive(Subcommand, Debug)]
enum BitcoinCommand {
    /// Attest a specific Bitcoin UTXO with three-family post-quantum signatures
    Attest {
        /// The UTXO in txid:vout format
        utxo: String,
        /// The Bitcoin address holding the UTXO
        #[arg(long)]
        address: String,
        /// Path to a file containing the signed proof-of-control message (JSON)
        #[arg(long)]
        proof: String,
    },
    /// Verify a previously-issued attestation (public, no auth required)
    Verify {
        /// The attestation ID
        attestation_id: String,
    },
    /// Find any H33 attestations for a given UTXO (public, no auth required)
    Lookup {
        /// The UTXO in txid:vout format
        utxo: String,
    },
}

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Signup => commands::signup::run().await,
        Commands::Mint { ttl, production, user, agent } => {
            commands::mint::run(&cli.api_base, ttl, production, &user, &agent).await
        }
        Commands::Mcp => commands::mcp::run(&cli.api_base).await,
        Commands::Status => commands::status::run(&cli.api_base).await,
        Commands::Audit { limit } => commands::audit::run(&cli.api_base, limit).await,
        Commands::Domains => commands::domains::run().await,
        Commands::Detect { path } => {
            commands::detect::run(path.as_deref().unwrap_or(".")).await
        }
        Commands::Wrap { file } => commands::wrap::run(&file),
        Commands::Verify { anchor_id } => {
            commands::verify::run(&cli.api_base, &anchor_id).await
        }
        Commands::Scan { path, baseline, diff } => {
            commands::scan::run(&cli.api_base, path.as_deref().unwrap_or("."), baseline, diff).await
        }
        Commands::Bitcoin { sub } => match sub {
            BitcoinCommand::Attest { utxo, address, proof } => {
                commands::bitcoin::attest(&cli.api_base, &utxo, &address, &proof).await
            }
            BitcoinCommand::Verify { attestation_id } => {
                commands::bitcoin::verify(&cli.api_base, &attestation_id).await
            }
            BitcoinCommand::Lookup { utxo } => {
                commands::bitcoin::lookup(&cli.api_base, &utxo).await
            }
        },
        Commands::Health => commands::health::run(&cli.api_base).await,
    }
}
