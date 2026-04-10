# h33-cli

**The H33 terminal companion — native Rust.**

Sign up for H33, mint short-lived agent capability tokens, run the H33 MCP
server for Claude Code / Cursor / Aider, audit your tenant, scan a codebase
for classical or quantum-vulnerable cryptography, wrap any payload with the
H33 substrate, attest a Bitcoin UTXO with three post-quantum signature
families, and check your live API health — all from one binary.

Replaces the legacy TypeScript CLI. Zero Node dependencies.

```
┌────────────────────────────────────────────────────────────┐
│  $ h33 signup                                              │
│  $ h33 mint                                                │
│  $ h33 mcp                ← spawns the H33 MCP server      │
│  $ h33 detect ./repo      ← scans for classical crypto     │
│  $ h33 wrap ./build.tar   ← anchors with the substrate     │
│  $ h33 bitcoin attest ... ← post-quantum UTXO insurance    │
│  $ h33 health                                              │
└────────────────────────────────────────────────────────────┘
```

---

## Install

### Via cargo

```bash
cargo install h33-cli
```

### Via Homebrew

```bash
brew install h33ai/tap/h33-cli
```

### Via one-command installer

```bash
curl -sSL https://install.h33.ai | sh
```

The installer detects your platform, downloads the signed Rust binary from
GitHub Releases, verifies its SHA-256, and places it on your `PATH` as `h33`.

---

## The architectural rule

> **Agents hold `cka_*`. Servers hold `ck_live_*`. They are never the same thing.**

`ck_live_*` production keys are server-side credentials. They never enter an
agent context. `cka_*` agent capability tokens are short-lived, scoped,
attributable, and what every AI agent uses to call H33. The `h33 mcp`
subcommand refuses to start the MCP server if it is given a `ck_live_*` key
as its token.

`h33 mint` is how you create a `cka_*` token from your `ck_live_*` server key
on your local terminal.

---

## Quickstart

```bash
# 1. Sign up (free tier, 10,000 auths/month, no credit card required)
h33 signup
# (opens https://h33.ai/signup in your browser)

# 2. Add your ck_live_* key from the dashboard to .env
echo 'H33_API_KEY=ck_live_...' >> .env

# 3. Mint a cka_* agent capability token (sandbox by default, 1 hour TTL)
h33 mint
# → export H33_AGENT_TOKEN=cka_AQAA...

# 4. Run the H33 MCP server for your AI coding agent
h33 mcp
```

---

## Subcommands

| Command | What it does |
|---|---|
| `h33 signup` | Open the H33 signup page in your browser |
| `h33 mint` | Mint a short-lived `cka_*` agent capability token |
| `h33 mcp` | Spawn the H33 MCP server (stdio) |
| `h33 status` | Show your tenant profile and usage |
| `h33 audit` | Tail your H33 audit log |
| `h33 domains` | List your substrate attestation domains |
| `h33 detect ./repo` | Scan a codebase for classical / quantum-vulnerable crypto |
| `h33 wrap <file>` | Wrap a payload with the H33 substrate (74-byte commitment) |
| `h33 verify <id>` | Verify a substrate attestation |
| `h33 scan ./repo` | Run an HICS code-quality scan |
| `h33 bitcoin attest ...` | Attest a Bitcoin UTXO with three post-quantum signature families |
| `h33 bitcoin verify <id>` | Verify a Bitcoin UTXO attestation |
| `h33 bitcoin lookup <utxo>` | Look up every H33 attestation for a given UTXO |
| `h33 health` | Check the H33 API health endpoint |

Run `h33 <command> --help` for full options.

---

## Configuration

`h33-cli` reads configuration from environment variables and/or a local
`.env` file (priority: env var → `.env` → `.env.local`).

| Variable | Purpose |
|---|---|
| `H33_API_KEY` | `ck_test_*` (sandbox) or `ck_live_*` (production) server key. Required for `mint`, `audit`, `domains`, `status`. |
| `H33_AGENT_TOKEN` | `cka_*` short-lived agent capability token. Required for `mcp`, `wrap`, and the Bitcoin subcommands. |
| `H33_API_BASE` | Override the H33 API endpoint. Defaults to `https://api.h33.ai`. |
| `H33_MCP_BIN` | Override the path to the `h33-mcp` binary that `h33 mcp` spawns. Defaults to `h33-mcp` on `PATH`. |

---

## Distinctions vs other H33 tools

- **`h33-cli`** (this crate) — the human terminal companion. Interactive,
  colored output, subcommand UX, designed for developers signing up + minting
  + running scans by hand.
- **[`h33-mcp`](https://crates.io/crates/h33-mcp)** — the Model Context
  Protocol server that AI coding agents (Claude Code, Cursor, Codex, Aider)
  spawn over stdio. `h33 mcp` is just a thin convenience wrapper that spawns
  it for you.
- **[`h33-agent-token`](https://crates.io/crates/h33-agent-token)** — the
  shared crate for the `cka_*` token format. You don't depend on this
  directly unless you're building your own H33 client.

---

## License

Proprietary — Commercial License Required. Sign up at
[h33.ai/signup](https://h33.ai/signup). The repository is open-source-readable
for research, audit, and reference-implementation purposes.

Patent pending — 129 claims filed.

---

## Resources

- Website: [h33.ai](https://h33.ai)
- Docs: [h33.ai/docs](https://h33.ai/docs)
- MCP server: [h33.ai/mcp](https://h33.ai/mcp)
- Bitcoin UTXO insurance: [h33.ai/bitcoin](https://h33.ai/bitcoin)
- Repository: [github.com/H33ai-postquantum/h33-cli-rs](https://github.com/H33ai-postquantum/h33-cli-rs)
- Support: support@h33.ai
