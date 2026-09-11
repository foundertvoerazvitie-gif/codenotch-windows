//! Grok (xAI Grok CLI / SuperGrok) usage adapter, implemented from the upstream Codenotch's
//! documented behaviour.
//!
//! Data path (same bargain as Claude Code / Codex: borrow the CLI's own session, never refresh):
//!   1. Credential: `~/.grok/auth.json`, keyed by `issuer::client_id`. Only sessions minted by
//!      `https://auth.x.ai` are trusted (a customer IdP token must not be sent to the public
//!      cli-chat-proxy). Entry fields: `key` (access token), `expires_at`, `email`.
//!   2. Endpoint: `GET https://cli-chat-proxy.grok.com/v1/billing?format=credits`
//!      Headers: `Authorization: Bearer <key>`, `X-XAI-Token-Auth: xai-grok-cli`, Accept JSON, 15 s.
//!      Reply `{ config: { currentPeriod, creditUsagePercent, productUsage[{product,usagePercent}],
//!                         billingPeriodStart, billingPeriodEnd } }` — credits is the weekly
//!      Grok Build allowance (the ring). A weekly plan pool that states `currentPeriod` but omits
//!      percentages until usage lands is shown as a "Weekly limit" bar at 0%, matching Grok's own
//!      `/usage`. An empty config is nothingMetered (status `none`), not a fabricated zero.
//!
//! Read only; tokens never reach logs, events or the UI. 401/403 → needsAuth; 429 → back off
//! 60 s × 2^n capped at 15 min (Retry-After only raises the floor); failures keep the last
//! reading marked stale.

use crate::usage::{LimitWindow, UsageSnapshot};
use crate::AppState;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};

const ENDPOINT: &str = "https://cli-chat-proxy.grok.com/v1/billing?format=credits";
const TRUSTED_ISSUER: &str = "https://auth.x.ai";
const POLL_SECS: u64 = 300;
const BACKOFF_BASE_SECS: u64 = 60;
const BACKOFF_CAP_SECS: u64 = 900;

static REFRESH: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static BACKOFF_UNTIL: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

pub fn request_refresh() {
    REFRESH.store(true, std::sync::atomic::Ordering::Relaxed);
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn grok_home() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".grok"))
}

fn auth_path() -> Option<PathBuf> {
    grok_home().map(|h| h.join("auth.json"))
}

fn store_path() -> PathBuf {
    crate::config::config_path().with_file_name("grok.json")
}

pub fn load_persisted() -> UsageSnapshot {
    std::fs::read_to_string(store_path())
        .ok()
        .and_then(|t| serde_json::from_str::<UsageSnapshot>(&t).ok())
        .map(|mut s| {
            if !s.windows.is_empty() {
                s.status = "stale".into();
            }
            BACKOFF_UNTIL.store(s.backoff_until, std::sync::atomic::Ordering::Relaxed);
            s
        })
        .unwrap_or_default()
}

fn persist(s: &UsageSnapshot) {
    if let Ok(t) = serde_json::to_string_pretty(s) {
        let _ = std::fs::write(store_path(), t);
    }
}

/// Installed / signed-in enough to warrant a cell (auth file or the CLI's home directory).
pub fn present() -> bool {
    auth_path().map(|p| p.is_file()).unwrap_or(false)
        || grok_home().map(|h| h.is_dir()).unwrap_or(false)
}

struct Credential {
    access_token: String,
    email: Option<String>,
    expired: bool,
}

fn is_trusted(key: &str, entry: &serde_json::Value) -> bool {
    if key.starts_with(TRUSTED_ISSUER) {
        return true;
    }
    entry
        .get("oidc_issuer")
        .and_then(|x| x.as_str())
        .map(|i| i == TRUSTED_ISSUER)
        .unwrap_or(false)
}

fn parse_expiry_ms(v: Option<&serde_json::Value>) -> Option<u64> {
    v.and_then(|x| x.as_str())
        .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.timestamp_millis().max(0) as u64)
}

/// Prefer a still-live trusted entry; otherwise the first trusted one.
fn pick_entry(root: &serde_json::Value) -> Option<&serde_json::Value> {
    let obj = root.as_object()?;
    let mut trusted: Vec<&serde_json::Value> = Vec::new();
    for (key, value) in obj {
        if value.is_object() && is_trusted(key, value) {
            trusted.push(value);
        }
    }
    let now = now_ms();
    if let Some(live) = trusted.iter().find(|e| {
        match parse_expiry_ms(e.get("expires_at")) {
            Some(exp) => exp > now,
            None => true,
        }
    }) {
        return Some(*live);
    }
    trusted.first().copied()
}

fn load_credential() -> Option<Credential> {
    let text = std::fs::read_to_string(auth_path()?).ok()?;
    let root: serde_json::Value = serde_json::from_str(&text).ok()?;
    let entry = pick_entry(&root)?;
    let access_token = entry.get("key")?.as_str()?.trim().to_string();
    if access_token.is_empty() {
        return None;
    }
    let expired = parse_expiry_ms(entry.get("expires_at"))
        .map(|exp| exp <= now_ms())
        .unwrap_or(false);
    let email = entry
        .get("email")
        .and_then(|x| x.as_str())
        .map(|s| s.to_string());
    Some(Credential {
        access_token,
        email,
        expired,
    })
}

/// For doctor: contains no secret values.
pub fn probe() -> String {
    let path = auth_path()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|| "~/.grok/auth.json".into());
    match load_credential() {
        Some(c) => format!(
            "Grok: auth usable (token {} chars{}, {})",
            c.access_token.len(),
            if c.expired { ", expired" } else { "" },
            c.email
                .as_ref()
                .map(|e| format!("email set ({} chars)", e.len()))
                .unwrap_or_else(|| "no email".into())
        ),
        None if auth_path().map(|p| p.is_file()).unwrap_or(false) => {
            format!("Grok: {path} present but no trusted xAI session (run `grok login`)")
        }
        None => format!("Grok: {path} not found (run `grok login`)"),
    }
}

// ---------------- Parsing ----------------

fn num(v: Option<&serde_json::Value>) -> Option<f64> {
    v.and_then(|x| x.as_f64())
}

fn percent_fraction(v: Option<&serde_json::Value>) -> Option<f64> {
    num(v).map(|p| (p / 100.0).clamp(0.0, 1.0))
}

fn parse_iso_ms(v: Option<&serde_json::Value>) -> Option<u64> {
    parse_expiry_ms(v)
}

/// "GrokBuild" → "Grok Build". The wire name is one word; Grok's usage modal writes two.
pub fn humanize(name: &str) -> String {
    let mut result = String::new();
    for ch in name.chars() {
        if ch.is_uppercase() && !result.is_empty() {
            result.push(' ');
        }
        result.push(ch);
    }
    result
}

fn product_label(credits: &serde_json::Value) -> Option<String> {
    credits
        .get("productUsage")
        .and_then(|x| x.as_array())
        .and_then(|arr| arr.first())
        .and_then(|p| p.get("product"))
        .and_then(|x| x.as_str())
        .map(humanize)
}

#[derive(Debug, PartialEq)]
pub enum ParseErr {
    BadResponse,
    NothingMetered,
}

/// Credits payload → LimitWindow(s). Pinned to the same fixtures as upstream GrokUsageTests.
pub fn windows_from_credits(root: &serde_json::Value) -> Result<Vec<LimitWindow>, ParseErr> {
    let credits = root
        .get("config")
        .filter(|c| c.is_object())
        .ok_or(ParseErr::BadResponse)?;

    let current_period = credits.get("currentPeriod").filter(|p| p.is_object());
    let current_end = current_period.and_then(|p| parse_iso_ms(p.get("end")));
    let credits_reset = current_end.or_else(|| parse_iso_ms(credits.get("billingPeriodEnd")));
    let start = if current_end.is_none() {
        parse_iso_ms(credits.get("billingPeriodStart"))
    } else {
        current_period.and_then(|p| parse_iso_ms(p.get("start")))
    };
    // duration is computed on macOS for the ring animation; Windows LimitWindow has no duration
    // field — resets_at alone drives the UI.
    let _duration_ms = match (start, credits_reset) {
        (Some(s), Some(e)) if e >= s => Some(e - s),
        _ => None,
    };

    let mut windows: Vec<LimitWindow> = Vec::new();

    if let Some(fraction) = percent_fraction(credits.get("creditUsagePercent")) {
        windows.push(LimitWindow {
            id: "credits".into(),
            label: product_label(credits).unwrap_or_else(|| "Grok Build".into()),
            used: fraction,
            resets_at: credits_reset,
            ..Default::default()
        });
    } else if let Some(products) = credits.get("productUsage").and_then(|x| x.as_array()) {
        for product in products {
            let Some(fraction) = percent_fraction(product.get("usagePercent")) else {
                continue;
            };
            let wire = product
                .get("product")
                .and_then(|x| x.as_str())
                .unwrap_or("Usage");
            let name = humanize(wire);
            let id = if windows.is_empty() {
                "credits".to_string()
            } else {
                wire.to_string()
            };
            windows.push(LimitWindow {
                id,
                label: name,
                used: fraction,
                resets_at: credits_reset,
                ..Default::default()
            });
        }
    }

    // Weekly pool with no percent yet → empty bar at 0%, not "unmetered".
    if windows.is_empty() {
        if let Some(weekly) = current_period {
            let typ = weekly.get("type").and_then(|x| x.as_str()).unwrap_or("");
            if typ.contains("WEEKLY") {
                windows.push(LimitWindow {
                    id: "credits".into(),
                    label: "Weekly limit".into(),
                    used: 0.0,
                    resets_at: parse_iso_ms(weekly.get("end")).or(credits_reset),
                    ..Default::default()
                });
            }
        }
    }

    if windows.is_empty() {
        return Err(ParseErr::NothingMetered);
    }
    Ok(windows)
}

// ---------------- Fetch ----------------

enum FetchErr {
    NeedsAuth,
    RateLimited(u64),
    Other(String),
}

fn fetch_once(token: &str) -> Result<serde_json::Value, FetchErr> {
    let resp = ureq::get(ENDPOINT)
        .set("Authorization", &format!("Bearer {token}"))
        .set("X-XAI-Token-Auth", "xai-grok-cli")
        .set("Accept", "application/json")
        .timeout(Duration::from_secs(15))
        .call();
    match resp {
        Ok(r) => r
            .into_json()
            .map_err(|e| FetchErr::Other(format!("parse: {e}"))),
        Err(ureq::Error::Status(401, _)) | Err(ureq::Error::Status(403, _)) => {
            Err(FetchErr::NeedsAuth)
        }
        Err(ureq::Error::Status(429, r)) => {
            let ra = r
                .header("retry-after")
                .and_then(|s| s.trim().parse::<u64>().ok())
                .unwrap_or(0);
            Err(FetchErr::RateLimited(ra))
        }
        Err(ureq::Error::Status(code, _)) => Err(FetchErr::Other(format!("HTTP {code}"))),
        Err(e) => Err(FetchErr::Other(format!("{e}"))),
    }
}

fn backoff_secs(consecutive: u32, retry_after_floor: u64) -> u64 {
    let exp = BACKOFF_BASE_SECS.saturating_mul(1u64 << consecutive.min(4));
    exp.clamp(BACKOFF_BASE_SECS, BACKOFF_CAP_SECS)
        .max(retry_after_floor)
}

fn read_once(prev: &UsageSnapshot, consecutive_429: &mut u32) -> UsageSnapshot {
    let mut snap = prev.clone();
    let held = BACKOFF_UNTIL.load(std::sync::atomic::Ordering::Relaxed);
    let now = now_ms();
    if held > now {
        snap.backoff_until = held;
        if !snap.windows.is_empty() {
            snap.status = "stale".into();
        }
        snap.note = format!(
            "Rate limited — retrying in {}s",
            (held - now) / 1000
        );
        return snap;
    }

    let Some(cred) = load_credential() else {
        snap.status = "needsAuth".into();
        snap.note = "Run grok login — it signs in and refreshes the token this reads.".into();
        return snap;
    };

    match fetch_once(&cred.access_token) {
        Ok(v) => match windows_from_credits(&v) {
            Ok(windows) => {
                *consecutive_429 = 0;
                snap.status = "ok".into();
                snap.windows = windows;
                snap.fetched_at = now_ms();
                snap.backoff_until = 0;
                snap.note = cred
                    .email
                    .map(|e| format!("{e} · via Grok"))
                    .unwrap_or_default();
            }
            Err(ParseErr::NothingMetered) => {
                *consecutive_429 = 0;
                snap.status = "none".into();
                snap.windows.clear();
                snap.fetched_at = now_ms();
                snap.backoff_until = 0;
                snap.note = "Grok has nothing metered on this account yet".into();
            }
            Err(ParseErr::BadResponse) => {
                snap.status = if snap.windows.is_empty() {
                    "error"
                } else {
                    "stale"
                }
                .into();
                snap.note = "Grok credits reply was not usable".into();
            }
        },
        Err(FetchErr::NeedsAuth) => {
            snap.status = "needsAuth".into();
            snap.note = if cred.expired {
                "Grok sign-in expired — run `grok login` to refresh it".into()
            } else {
                "Grok rejected its sign-in — run `grok login` again".into()
            };
        }
        Err(FetchErr::RateLimited(ra)) => {
            *consecutive_429 = consecutive_429.saturating_add(1);
            let wait = backoff_secs(consecutive_429.saturating_sub(1), ra);
            let until = now_ms() + wait * 1000;
            BACKOFF_UNTIL.store(until, std::sync::atomic::Ordering::Relaxed);
            snap.backoff_until = until;
            if !snap.windows.is_empty() {
                snap.status = "stale".into();
            }
            snap.note = format!("Rate limited — retrying in {wait}s");
        }
        Err(FetchErr::Other(msg)) => {
            snap.status = if snap.windows.is_empty() {
                "error"
            } else {
                "stale"
            }
            .into();
            snap.note = msg;
        }
    }
    snap
}

fn broadcast(app: &AppHandle, snap: UsageSnapshot) {
    let st = app.state::<AppState>();
    *st.grok.lock().unwrap() = snap.clone();
    persist(&snap);
    let _ = app.emit("grok", &snap);
    crate::threshold::observe(app, "grok", "Grok", &snap);
}

fn sleep_interruptible(secs: u64) {
    for _ in 0..secs {
        if REFRESH.swap(false, std::sync::atomic::Ordering::Relaxed) {
            return;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}

pub fn start(app: AppHandle) {
    std::thread::spawn(move || {
        {
            let st = app.state::<AppState>();
            let snap = st.grok.lock().unwrap().clone();
            let _ = app.emit("grok", &snap);
        }
        if !present() {
            broadcast(
                &app,
                UsageSnapshot {
                    status: "absent".into(),
                    ..Default::default()
                },
            );
            loop {
                sleep_interruptible(600);
                if present() {
                    break;
                }
            }
        }
        let mut consecutive_429: u32 = 0;
        loop {
            let prev = {
                let st = app.state::<AppState>();
                st.grok.lock().unwrap().clone()
            };
            let snap = read_once(&prev, &mut consecutive_429);
            if snap.status == "error" || snap.status == "stale" {
                crate::applog(&format!("grok: {}", snap.note));
            }
            let hold = snap.backoff_until.saturating_sub(now_ms()) / 1000;
            broadcast(&app, snap);
            sleep_interruptible(POLL_SECS.max(hold));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn credits_fixture() -> serde_json::Value {
        serde_json::from_str(
            r#"{
              "config": {
                "currentPeriod": {
                  "type": "USAGE_PERIOD_TYPE_WEEKLY",
                  "start": "2026-09-05T08:21:18.802818+00:00",
                  "end": "2026-09-12T08:21:18.802818+00:00"
                },
                "creditUsagePercent": 8.0,
                "onDemandCap": {"val": 0},
                "onDemandUsed": {"val": 0},
                "productUsage": [{"product": "GrokBuild", "usagePercent": 8.0}],
                "isUnifiedBillingUser": true,
                "prepaidBalance": {"val": 0},
                "topUpMethod": "TOP_UP_METHOD_SAVED_PAYMENT_METHOD",
                "billingPeriodStart": "2026-09-05T08:21:18.802818+00:00",
                "billingPeriodEnd": "2026-09-12T08:21:18.802818+00:00"
              }
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn the_ring_is_the_credits_percentage() {
        let w = windows_from_credits(&credits_fixture()).unwrap();
        let credits = w.iter().find(|x| x.id == "credits").unwrap();
        assert_eq!(credits.label, "Grok Build");
        assert!((credits.used - 0.08).abs() < 0.0001);
        let reset = credits.resets_at.unwrap();
        let dt = chrono::DateTime::from_timestamp_millis(reset as i64).unwrap();
        assert_eq!(dt.month(), 9);
        assert_eq!(dt.day(), 12);
    }

    #[test]
    fn an_empty_config_is_not_a_successful_reading() {
        let v: serde_json::Value = serde_json::from_str(r#"{"config":{}}"#).unwrap();
        assert_eq!(windows_from_credits(&v).unwrap_err(), ParseErr::NothingMetered);
    }

    #[test]
    fn product_only_credits_still_use_the_headline_id() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{
              "config": {
                "productUsage": [{"product": "GrokBuild", "usagePercent": 33.0}],
                "billingPeriodEnd": "2026-09-12T08:21:18.802818+00:00"
              }
            }"#,
        )
        .unwrap();
        let w = windows_from_credits(&v).unwrap();
        let credits = w.iter().find(|x| x.id == "credits").unwrap();
        assert_eq!(credits.label, "Grok Build");
        assert!((credits.used - 0.33).abs() < 0.0001);
    }

    #[test]
    fn weekly_pool_without_a_percent_is_a_zero_ring() {
        let v: serde_json::Value = serde_json::from_str(
            r#"{
              "config": {
                "currentPeriod": {
                  "type": "USAGE_PERIOD_TYPE_WEEKLY",
                  "start": "2026-09-07T20:59:12+00:00",
                  "end": "2026-09-14T20:59:12+00:00"
                },
                "onDemandCap": {"val": 0},
                "onDemandUsed": {"val": 0},
                "isUnifiedBillingUser": true,
                "prepaidBalance": {"val": 0}
              }
            }"#,
        )
        .unwrap();
        let w = windows_from_credits(&v).unwrap();
        let credits = w.iter().find(|x| x.id == "credits").unwrap();
        assert_eq!(credits.label, "Weekly limit");
        assert!((credits.used - 0.0).abs() < 0.0001);
        let reset = credits.resets_at.unwrap();
        let dt = chrono::DateTime::from_timestamp_millis(reset as i64).unwrap();
        assert_eq!(dt.month(), 9);
        assert_eq!(dt.day(), 14);
    }

    #[test]
    fn garbage_is_a_bad_response_rather_than_a_guess() {
        let v: serde_json::Value = serde_json::from_str(r#""not an object""#).unwrap();
        assert_eq!(windows_from_credits(&v).unwrap_err(), ParseErr::BadResponse);
    }

    #[test]
    fn humanizes_the_product_name_the_way_the_modal_writes_it() {
        assert_eq!(humanize("GrokBuild"), "Grok Build");
    }

    use chrono::Datelike;
}
