//! `h33 upgrade` — upgrade to a paid plan via Stripe.
//!
//! Shows current plan, lists available tiers, opens Stripe checkout.

use crate::{client::join_url, config, output};
use anyhow::{Context, Result};
use colored::Colorize;

const CHECKOUT_ENDPOINT: &str = "/api/h33/subscription/checkout";
const PLAN_ENDPOINT: &str = "/api/h33/subscription/plan";

struct PlanInfo {
    slug: &'static str,
    name: &'static str,
    units: &'static str,
    price: &'static str,
}

const PLANS: &[PlanInfo] = &[
    PlanInfo { slug: "starter",  name: "Starter",    units: "5,000/mo",   price: "$349/mo" },
    PlanInfo { slug: "pro",      name: "Pro",        units: "15,000/mo",  price: "$899/mo" },
    PlanInfo { slug: "business", name: "Business",   units: "50,000/mo",  price: "$2,499/mo" },
    PlanInfo { slug: "growth",   name: "Growth",     units: "175,000/mo", price: "$6,999/mo" },
    PlanInfo { slug: "scale",    name: "Scale",      units: "500,000/mo", price: "$17,999/mo" },
];

pub async fn run(auth_base: &str, tier: Option<&str>) -> Result<()> {
    output::banner();
    println!();

    let token = config::require_api_key()?;

    let client = reqwest::Client::builder()
        .user_agent(concat!("h33-cli/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(15))
        .no_proxy()
        .build()
        .context("building HTTP client")?;

    // Auth1 direct (bypass Netlify WAF)
    let base = if auth_base == "https://h33.ai" || auth_base == "https://api.h33.ai" {
        "https://auth-api.z101.ai"
    } else {
        auth_base
    };

    // Show current plan
    let plan_resp = client
        .get(join_url(base, PLAN_ENDPOINT))
        .header("Authorization", format!("Bearer {token}"))
        .send()
        .await;

    if let Ok(resp) = plan_resp {
        if resp.status().is_success() {
            if let Ok(data) = resp.json::<serde_json::Value>().await {
                let current_tier = data.get("tier").and_then(|v| v.as_str()).unwrap_or("free");
                let quota = data.get("monthly_quota").and_then(|v| v.as_u64()).unwrap_or(0);
                let used = data.get("period_usage").and_then(|v| v.as_u64()).unwrap_or(0);
                let remaining = data.get("remaining").and_then(|v| v.as_u64()).unwrap_or(0);

                println!("  {}", "Current Plan".bold().underline());
                println!("  Tier:      {}", current_tier.bold());
                println!("  Quota:     {} units/mo", quota);
                println!("  Used:      {}", used);
                println!("  Remaining: {}", remaining);
                println!();
            }
        }
    }

    // If no tier specified, show options
    let selected_tier = match tier {
        Some(t) => t.to_string(),
        None => {
            println!("  {}", "Available Plans".bold().underline());
            println!();
            for (i, plan) in PLANS.iter().enumerate() {
                println!(
                    "  {}. {} — {} — {}",
                    (i + 1).to_string().bold(),
                    plan.name.bold(),
                    plan.units.bright_black(),
                    plan.price
                );
            }
            println!();
            println!("  Enterprise: contact sales@h33.ai");
            println!();
            println!(
                "  Run: {} to subscribe",
                "h33 upgrade --tier starter".bold()
            );
            println!();
            return Ok(());
        }
    };

    // Validate tier
    if !PLANS.iter().any(|p| p.slug == selected_tier) {
        anyhow::bail!(
            "Invalid tier '{}'. Valid: {}",
            selected_tier,
            PLANS.iter().map(|p| p.slug).collect::<Vec<_>>().join(", ")
        );
    }

    println!(
        "  {} Creating checkout for {} plan...",
        "→".bright_black(),
        selected_tier.bold()
    );

    // Create Stripe checkout
    let resp = client
        .post(join_url(base, CHECKOUT_ENDPOINT))
        .header("Authorization", format!("Bearer {token}"))
        .json(&serde_json::json!({ "tier": selected_tier }))
        .send()
        .await
        .context("Stripe checkout request failed")?;

    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        anyhow::bail!("Checkout failed: {}", body);
    }

    let data: serde_json::Value = resp.json().await.context("parsing checkout response")?;
    let checkout_url = data.get("checkout_url").and_then(|v| v.as_str()).unwrap_or("");
    let monthly_units = data.get("monthly_units").and_then(|v| v.as_u64()).unwrap_or(0);

    if checkout_url.is_empty() {
        anyhow::bail!("No checkout URL returned");
    }

    println!();
    println!(
        "  {} {} plan — {} units/mo",
        "✔".green().bold(),
        selected_tier.bold(),
        monthly_units
    );
    println!();
    println!("  Opening checkout in browser...");
    println!();

    // Open in browser
    if let Err(_) = open::that(checkout_url) {
        println!("  Could not open browser. Visit:");
        println!("  {}", checkout_url);
    }

    println!(
        "  {} Complete payment in your browser, then run {} to verify.",
        "→".bright_black(),
        "h33 status".bold()
    );
    println!();

    Ok(())
}
