//! Shared HTTP client for H33 API calls.
//!
//! Supports two auth modes:
//! - X-API-Key header (for h33_live_* and h33_sandbox_* keys from `h33 init`)
//! - Authorization: Bearer (for cka_* agent tokens from `h33 mint`)

use anyhow::{anyhow, Context, Result};
use reqwest::{Client, Method};
use serde_json::Value;
use std::time::Duration;

/// Auth mode for API calls.
pub enum Auth<'a> {
    /// X-API-Key header (API keys: h33_live_*, h33_sandbox_*, ck_*)
    ApiKey(&'a str),
    /// Authorization: Bearer header (agent tokens: cka_*)
    Bearer(&'a str),
    /// No auth
    None,
}

pub struct H33Client {
    api_base: String,
    client: Client,
}

/// Join a base URL and a path, avoiding double slashes and double path prefixes.
///
/// Rules:
/// - Base trailing slash is stripped
/// - Path must start with `/`
/// - If base already ends with a path prefix that path starts with, no duplication
///
/// Examples:
///   ("https://h33.ai", "/api/h33/keys") → "https://h33.ai/api/h33/keys"
///   ("https://h33.ai/", "/health") → "https://h33.ai/health"
///   ("https://api.h33.ai", "/v1/hics/scan") → "https://api.h33.ai/v1/hics/scan"
pub fn join_url(base: &str, path: &str) -> String {
    let base = base.trim_end_matches('/');
    let path = if path.starts_with('/') { path } else { return format!("{base}/{path}") };
    format!("{base}{path}")
}

impl H33Client {
    pub fn new(api_base: impl Into<String>) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("h33-cli/", env!("CARGO_PKG_VERSION")))
            .build()
            .context("building HTTP client")?;
        Ok(Self {
            api_base: api_base.into(),
            client,
        })
    }

    fn url(&self, path: &str) -> String {
        join_url(&self.api_base, path)
    }

    /// POST with JSON body, using the best available auth.
    pub async fn post_json(&self, path: &str, bearer: &str, body: Value) -> Result<Value> {
        // Detect auth type from token prefix
        let auth = detect_auth(bearer);
        self.request(Method::POST, path, auth, Some(body)).await
    }

    /// GET with optional auth.
    pub async fn get_json(&self, path: &str, bearer: Option<&str>) -> Result<Value> {
        let auth = bearer.map(|b| detect_auth(b)).unwrap_or(Auth::None);
        self.request(Method::GET, path, auth, None).await
    }

    /// POST with explicit Auth enum.
    pub async fn post_with_auth(&self, path: &str, auth: Auth<'_>, body: Value) -> Result<Value> {
        self.request(Method::POST, path, auth, Some(body)).await
    }

    /// GET with explicit Auth enum.
    pub async fn get_with_auth(&self, path: &str, auth: Auth<'_>) -> Result<Value> {
        self.request(Method::GET, path, auth, None).await
    }

    async fn request(
        &self,
        method: Method,
        path: &str,
        auth: Auth<'_>,
        body: Option<Value>,
    ) -> Result<Value> {
        let url = self.url(path);
        let mut req = self.client.request(method, &url);

        match auth {
            Auth::ApiKey(key) => {
                req = req.header("X-API-Key", key);
            }
            Auth::Bearer(token) => {
                req = req.bearer_auth(token);
            }
            Auth::None => {}
        }

        if let Some(body_val) = body {
            req = req.json(&body_val);
        }

        let res = req
            .send()
            .await
            .with_context(|| format!("request to {}", url))?;
        let status = res.status();
        if !status.is_success() && status.as_u16() != 204 {
            let body_text = res.text().await.unwrap_or_default();
            return Err(anyhow!(
                "H33 API {} {} → {}: {}",
                status.as_u16(),
                url,
                status.canonical_reason().unwrap_or("error"),
                body_text.chars().take(200).collect::<String>()
            ));
        }
        if status.as_u16() == 204 {
            return Ok(Value::Null);
        }
        let value = res.json::<Value>().await.unwrap_or(Value::Null);
        Ok(value)
    }
}

/// Detect auth type from token prefix.
/// h33_live_*, h33_sandbox_*, ck_live_*, ck_test_* → X-API-Key
/// cka_* → Bearer
/// anything else → Bearer (legacy default)
fn detect_auth(token: &str) -> Auth<'_> {
    if token.starts_with("h33_") || token.starts_with("ck_") {
        Auth::ApiKey(token)
    } else if token.starts_with("cka_") {
        Auth::Bearer(token)
    } else {
        // Legacy: treat unknown tokens as Bearer for backward compat
        Auth::Bearer(token)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_join_url_basic() {
        assert_eq!(
            join_url("https://h33.ai", "/api/h33/keys/sandbox"),
            "https://h33.ai/api/h33/keys/sandbox"
        );
    }

    #[test]
    fn test_join_url_trailing_slash() {
        assert_eq!(
            join_url("https://h33.ai/", "/api/h33/keys/sandbox"),
            "https://h33.ai/api/h33/keys/sandbox"
        );
    }

    #[test]
    fn test_join_url_no_leading_slash() {
        assert_eq!(
            join_url("https://api.h33.ai", "v1/hics/scan"),
            "https://api.h33.ai/v1/hics/scan"
        );
    }

    #[test]
    fn test_join_url_gateway_paths() {
        assert_eq!(
            join_url("https://api.h33.ai", "/v1/hics/scan"),
            "https://api.h33.ai/v1/hics/scan"
        );
        assert_eq!(
            join_url("https://api.h33.ai", "/v1/substrate/verify"),
            "https://api.h33.ai/v1/substrate/verify"
        );
        assert_eq!(
            join_url("https://api.h33.ai", "/v1/agent_tokens"),
            "https://api.h33.ai/v1/agent_tokens"
        );
    }

    #[test]
    fn test_join_url_auth_paths() {
        assert_eq!(
            join_url("https://h33.ai", "/api/h33/keys/sandbox"),
            "https://h33.ai/api/h33/keys/sandbox"
        );
        assert_eq!(
            join_url("https://h33.ai", "/api/h33/keys"),
            "https://h33.ai/api/h33/keys"
        );
    }

    #[test]
    fn test_join_url_staging_override() {
        assert_eq!(
            join_url("https://staging.api.h33.ai", "/v1/hics/scan"),
            "https://staging.api.h33.ai/v1/hics/scan"
        );
        assert_eq!(
            join_url("https://staging.h33.ai", "/api/h33/keys/sandbox"),
            "https://staging.h33.ai/api/h33/keys/sandbox"
        );
    }

    #[test]
    fn test_join_url_health() {
        assert_eq!(
            join_url("https://api.h33.ai", "/health"),
            "https://api.h33.ai/health"
        );
    }

    #[test]
    fn test_join_url_no_double_slash() {
        // Trailing slash on base + leading slash on path = no double slash
        assert_eq!(
            join_url("https://h33.ai/", "/api/status"),
            "https://h33.ai/api/status"
        );
    }

    #[test]
    fn test_detect_auth_api_key() {
        assert!(matches!(detect_auth("h33_live_abc123"), Auth::ApiKey(_)));
        assert!(matches!(detect_auth("h33_sand_abc123"), Auth::ApiKey(_)));
        assert!(matches!(detect_auth("ck_live_abc"), Auth::ApiKey(_)));
    }

    #[test]
    fn test_detect_auth_bearer() {
        assert!(matches!(detect_auth("cka_abc123"), Auth::Bearer(_)));
    }

    #[test]
    fn test_detect_auth_jwt_fallback() {
        // JWTs and unknown tokens → Bearer
        assert!(matches!(detect_auth("eyJhbGciOiJIUzI1NiJ9.xxx"), Auth::Bearer(_)));
    }
}
