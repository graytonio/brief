/// Authentication — token storage, provider trait, and GitHub Device Flow.
///
/// Token resolution order (highest to lowest priority):
///   1. `CLAUDE_STANDARDS_TOKEN` environment variable (raw PAT, any provider)
///   2. `~/.brief/.token` — JSON `StoredToken` or legacy raw string
///
/// Adding a new provider:
///   1. Implement `AuthProvider` for a new struct.
///   2. Add a match arm in `cmd_auth_login` in `main.rs`.
///   3. No changes needed here or in `cache.rs`.
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::config;

// ─────────────────────────────────────────────────────────────────────────────
// StoredToken — what we persist in ~/.brief/.token
// ─────────────────────────────────────────────────────────────────────────────

/// A token persisted to `~/.brief/.token`.
///
/// The file is written as JSON. For backward compatibility, if the file
/// contains a plain string (old PAT workflow), `read_stored_token` wraps
/// it in a synthetic `StoredToken` rather than failing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredToken {
    /// Provider identifier: "github", "gitlab", "pat", …
    pub provider: String,
    /// "oauth" for Device Flow tokens, "pat" for personal access tokens.
    pub token_type: String,
    /// The raw bearer value sent as `Authorization: Bearer <token>`.
    pub token: String,
    /// OAuth scopes granted, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// Unix timestamp of when the token was stored.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub issued_at: Option<u64>,
}

/// Read the stored token from `~/.brief/.token`.
///
/// Returns `None` if the file does not exist or is empty.
/// If the file is not valid JSON, the raw content is treated as a legacy PAT.
pub fn read_stored_token() -> Option<StoredToken> {
    let path = config::token_path().ok()?;
    let contents = fs::read_to_string(path).ok()?;
    let trimmed = contents.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Try JSON first; fall back to treating the raw string as a PAT.
    match serde_json::from_str::<StoredToken>(trimmed) {
        Ok(t) => Some(t),
        Err(_) => Some(StoredToken {
            provider: "pat".into(),
            token_type: "pat".into(),
            token: trimmed.to_string(),
            scope: None,
            issued_at: None,
        }),
    }
}

/// Persist a token to `~/.brief/.token` (mode 600).
pub fn write_stored_token(t: &StoredToken) -> Result<()> {
    let path = config::token_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(t).context("Failed to serialize token")?;
    fs::write(&path, &json)
        .with_context(|| format!("Failed to write token file: {}", path.display()))?;

    // Restrict permissions on Unix so the token isn't world-readable.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .with_context(|| format!("Failed to set permissions on {}", path.display()))?;
    }

    Ok(())
}

/// Delete `~/.brief/.token`, logging out the current session.
pub fn delete_stored_token() -> Result<()> {
    let path = config::token_path()?;
    if path.exists() {
        fs::remove_file(&path)
            .with_context(|| format!("Failed to delete token file: {}", path.display()))?;
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// resolve_token — unified entry point used by cache::read_token
// ─────────────────────────────────────────────────────────────────────────────

/// Returns the raw bearer token string to attach to HTTP requests.
///
/// Resolution order:
///   1. `CLAUDE_STANDARDS_TOKEN` env var
///   2. `~/.brief/.token`
pub fn resolve_token() -> Option<String> {
    if let Ok(val) = std::env::var("CLAUDE_STANDARDS_TOKEN") {
        if !val.is_empty() {
            return Some(val);
        }
    }
    read_stored_token().map(|t| t.token)
}

// ─────────────────────────────────────────────────────────────────────────────
// AuthProvider trait
// ─────────────────────────────────────────────────────────────────────────────

/// Implemented by each auth provider (GitHub, GitLab, …).
#[allow(dead_code)]
pub trait AuthProvider {
    /// Human-readable name shown in `brief auth status`.
    fn name(&self) -> &str;

    /// Short identifier stored in `StoredToken::provider`.
    fn provider_id(&self) -> &str;

    /// Run the interactive login flow and return a ready-to-store token.
    fn login(&self) -> Result<StoredToken>;

    /// Optionally revoke the token server-side before local deletion.
    /// Default is a no-op; providers that support revocation can override.
    fn logout(&self, _token: &StoredToken) -> Result<()> {
        Ok(())
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// GitHub Device Flow
// ─────────────────────────────────────────────────────────────────────────────

/// Replace with your registered GitHub OAuth App client ID.
/// Only the client ID is needed for Device Flow — no secret required.
pub const GITHUB_CLIENT_ID: &str = "Ov23li0zLJyPMKdJtmn9";

const GITHUB_DEVICE_CODE_URL: &str = "https://github.com/login/device/code";
const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";
const GITHUB_SCOPE: &str = "repo";
/// Maximum time (seconds) to wait for the user to authorize before giving up.
const DEVICE_FLOW_TIMEOUT_SECS: u64 = 300;

pub struct GitHubDeviceFlow;

impl AuthProvider for GitHubDeviceFlow {
    fn name(&self) -> &str {
        "GitHub"
    }

    fn provider_id(&self) -> &str {
        "github"
    }

    fn login(&self) -> Result<StoredToken> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .context("Failed to build HTTP client")?;

        // ── Step 1: request device + user codes ──────────────────────────────
        let resp = client
            .post(GITHUB_DEVICE_CODE_URL)
            .header("Accept", "application/json")
            .form(&[("client_id", GITHUB_CLIENT_ID), ("scope", GITHUB_SCOPE)])
            .send()
            .context("Failed to reach GitHub")?;

        if !resp.status().is_success() {
            anyhow::bail!("GitHub returned HTTP {}", resp.status());
        }

        #[derive(Deserialize)]
        struct DeviceCodeResponse {
            device_code: String,
            user_code: String,
            verification_uri: String,
            expires_in: u64,
            interval: u64,
        }

        let dc: DeviceCodeResponse = resp.json().context("Failed to parse device code response")?;

        // ── Step 2: prompt user ───────────────────────────────────────────────
        println!();
        println!("  Copy your one-time code:  {}", dc.user_code);
        println!("  Then open:                {}", dc.verification_uri);
        println!();

        // Try to open the browser; failure is non-fatal.
        if let Err(e) = open::that(&dc.verification_uri) {
            eprintln!("brief: could not open browser automatically: {}", e);
        }

        println!("Waiting for authorization...");

        // ── Step 3: poll for token ────────────────────────────────────────────
        let deadline = now_secs() + dc.expires_in.min(DEVICE_FLOW_TIMEOUT_SECS);
        let mut poll_interval = dc.interval.max(5);

        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: Option<String>,
            token_type: Option<String>,
            scope: Option<String>,
            error: Option<String>,
        }

        loop {
            if now_secs() >= deadline {
                anyhow::bail!("Device flow timed out. Run `brief auth login` to try again.");
            }

            std::thread::sleep(Duration::from_secs(poll_interval));

            let poll = client
                .post(GITHUB_TOKEN_URL)
                .header("Accept", "application/json")
                .form(&[
                    ("client_id", GITHUB_CLIENT_ID),
                    ("device_code", dc.device_code.as_str()),
                    (
                        "grant_type",
                        "urn:ietf:params:oauth:grant-type:device_code",
                    ),
                ])
                .send()
                .context("Failed to poll GitHub for token")?;

            let tr: TokenResponse = poll.json().context("Failed to parse token poll response")?;

            if let Some(token) = tr.access_token {
                let issued_at = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                return Ok(StoredToken {
                    provider: self.provider_id().into(),
                    token_type: tr.token_type.unwrap_or_else(|| "oauth".into()),
                    token,
                    scope: tr.scope,
                    issued_at: Some(issued_at),
                });
            }

            match tr.error.as_deref() {
                Some("authorization_pending") => { /* keep polling */ }
                Some("slow_down") => {
                    poll_interval += 5;
                }
                Some("expired_token") => {
                    anyhow::bail!("Device code expired. Run `brief auth login` to try again.");
                }
                Some("access_denied") => {
                    anyhow::bail!("Authorization was denied.");
                }
                Some(other) => {
                    anyhow::bail!("GitHub returned error: {}", other);
                }
                None => { /* keep polling */ }
            }
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
