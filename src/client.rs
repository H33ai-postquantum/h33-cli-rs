//! Shared HTTP client for H33 API calls.

use anyhow::{anyhow, Context, Result};
use reqwest::{Client, Method};
use serde_json::Value;
use std::time::Duration;

pub struct H33Client {
    api_base: String,
    client: Client,
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
        format!("{}{}", self.api_base.trim_end_matches('/'), path)
    }

    pub async fn post_json(&self, path: &str, bearer: &str, body: Value) -> Result<Value> {
        self.request(Method::POST, path, Some(bearer), Some(body))
            .await
    }

    pub async fn get_json(&self, path: &str, bearer: Option<&str>) -> Result<Value> {
        self.request(Method::GET, path, bearer, None).await
    }

    async fn request(
        &self,
        method: Method,
        path: &str,
        bearer: Option<&str>,
        body: Option<Value>,
    ) -> Result<Value> {
        let url = self.url(path);
        let mut req = self.client.request(method, &url);
        if let Some(b) = bearer {
            req = req.bearer_auth(b);
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
