/// Fetch-with-cache logic and TTL handling.
///
/// Cache files live in ~/.brief/cache/.
/// Each URL is hashed (SHA-256) to produce a stable filename.
/// A companion .ts file records the Unix timestamp of the last successful fetch.
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config;

/// Returns the cache file path for a given URL.
pub fn cache_file(url: &str) -> Result<PathBuf> {
    let hash = url_hash(url);
    Ok(config::cache_dir()?.join(&hash))
}

/// Returns the timestamp sidecar path for a given URL.
pub fn timestamp_file(url: &str) -> Result<PathBuf> {
    let hash = url_hash(url);
    Ok(config::cache_dir()?.join(format!("{}.ts", hash)))
}

/// Computes the hex-encoded SHA-256 hash of a URL string.
fn url_hash(url: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(url.as_bytes());
    hex::encode(hasher.finalize())
}

/// Returns the current Unix timestamp in seconds.
fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Returns the Unix timestamp stored in the .ts sidecar, or None.
pub fn cached_timestamp(url: &str) -> Option<u64> {
    let ts_path = timestamp_file(url).ok()?;
    let contents = fs::read_to_string(ts_path).ok()?;
    contents.trim().parse::<u64>().ok()
}

/// Returns true if a valid (non-expired) cache entry exists for the given URL.
pub fn is_cache_fresh(url: &str, ttl_secs: u64) -> bool {
    match cached_timestamp(url) {
        Some(ts) => now_secs().saturating_sub(ts) < ttl_secs,
        None => false,
    }
}

/// Returns the cached content for a URL if any cache file exists (regardless of age).
pub fn read_cache(url: &str) -> Option<String> {
    let path = cache_file(url).ok()?;
    fs::read_to_string(path).ok()
}

/// Writes content to the cache and records the current timestamp.
pub fn write_cache(url: &str, content: &str) -> Result<()> {
    let dir = config::cache_dir()?;
    fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create cache directory: {}", dir.display()))?;

    let path = cache_file(url)?;
    fs::write(&path, content)
        .with_context(|| format!("Failed to write cache file: {}", path.display()))?;

    let ts_path = timestamp_file(url)?;
    fs::write(&ts_path, now_secs().to_string())
        .with_context(|| format!("Failed to write timestamp file: {}", ts_path.display()))?;

    Ok(())
}

/// Invalidates the cache for a URL by deleting the timestamp sidecar.
/// The cached content itself is kept so stale-cache fallback still works.
pub fn invalidate_cache(url: &str) -> Result<()> {
    let ts_path = timestamp_file(url)?;
    if ts_path.exists() {
        fs::remove_file(&ts_path)
            .with_context(|| format!("Failed to remove timestamp file: {}", ts_path.display()))?;
    }
    Ok(())
}

/// Reads the optional auth token from env var or ~/.brief/.token.
pub fn read_token() -> Option<String> {
    // 1. Environment variable takes precedence.
    if let Ok(token) = std::env::var("CLAUDE_STANDARDS_TOKEN") {
        if !token.is_empty() {
            return Some(token);
        }
    }
    // 2. Fall back to token file.
    let path = config::token_path().ok()?;
    let contents = fs::read_to_string(path).ok()?;
    let token = contents.trim().to_string();
    if token.is_empty() {
        None
    } else {
        Some(token)
    }
}

/// Fetches the given URL, sending an auth token if one is configured.
/// Returns the response body as a String on success.
pub fn fetch_url(url: &str) -> Result<String, FetchError> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| FetchError::Network(e.to_string()))?;

    let mut req = client.get(url);
    if let Some(token) = read_token() {
        req = req.header("Authorization", format!("Bearer {}", token));
    }

    let resp = req.send().map_err(|e| FetchError::Network(e.to_string()))?;
    if !resp.status().is_success() {
        return Err(FetchError::Http(resp.status().as_u16()));
    }
    resp.text().map_err(|e| FetchError::Network(e.to_string()))
}

/// Errors that can occur during an HTTP fetch.
#[derive(Debug, thiserror::Error)]
pub enum FetchError {
    #[error("Network error: {0}")]
    Network(String),
    #[error("HTTP error: status {0}")]
    Http(u16),
}

/// Fetches the content for a URL, honouring TTL and cache fallback.
///
/// Returns:
/// - `Ok(content)` — fresh or cached content
/// - `Err(...)` — fetch failed and no cache exists
///
/// If the fetch fails but a stale cache exists, logs a warning to stderr and
/// returns the stale content as Ok.
pub fn fetch_with_cache(url: &str, ttl_secs: u64) -> Result<String> {
    // Fast path: cache is fresh.
    if is_cache_fresh(url, ttl_secs) {
        if let Some(content) = read_cache(url) {
            return Ok(content);
        }
    }

    // Try network fetch.
    match fetch_url(url) {
        Ok(content) => {
            // Update cache on success.
            if let Err(e) = write_cache(url, &content) {
                eprintln!("brief: warning: failed to write cache: {}", e);
            }
            Ok(content)
        }
        Err(e) => {
            // Fall back to stale cache with a warning.
            if let Some(stale) = read_cache(url) {
                eprintln!(
                    "brief: warning: fetch failed ({}), using stale cache for {}",
                    e, url
                );
                Ok(stale)
            } else {
                Err(anyhow::anyhow!(
                    "Fetch failed and no cache available for {}: {}",
                    url,
                    e
                ))
            }
        }
    }
}

/// Age of the cached content in seconds, or None if never cached.
pub fn cache_age_secs(url: &str) -> Option<u64> {
    cached_timestamp(url).map(|ts| now_secs().saturating_sub(ts))
}
