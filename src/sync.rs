/// sync command — fetch and merge a remote team config.
///
/// The remote config is a TOML file with the same schema as the local config.
/// Merge rules:
///   - Remote languages not present locally are added.
///   - Remote languages already present locally are skipped (unless --force).
///   - The global URL is updated only if --force is passed.
use anyhow::{Context, Result};

use crate::cache;
use crate::config::Config;

/// Options for the sync operation.
pub struct SyncOptions {
    pub force: bool,
    pub dry_run: bool,
}

/// Fetches the remote team config from `url` and merges it into `local`.
///
/// Returns a description of what was (or would be) changed.
pub fn run_sync(url: &str, local: &mut Config, opts: &SyncOptions) -> Result<Vec<String>> {
    let raw = cache::fetch_url(url)
        .with_context(|| format!("Failed to fetch team config from {}", url))?;

    let remote: Config = toml::from_str(&raw)
        .with_context(|| format!("Failed to parse remote team config from {}", url))?;

    let mut changes: Vec<String> = Vec::new();

    // Merge global URL.
    if let Some(remote_url) = &remote.global.url {
        match &local.global.url {
            None => {
                changes.push(format!("Set global URL to {}", remote_url));
                if !opts.dry_run {
                    local.global.url = Some(remote_url.clone());
                }
            }
            Some(existing) if opts.force && existing != remote_url => {
                changes.push(format!("Update global URL: {} → {}", existing, remote_url));
                if !opts.dry_run {
                    local.global.url = Some(remote_url.clone());
                }
            }
            Some(existing) if !opts.force && existing != remote_url => {
                changes
                    .push("Skip global URL (local differs; use --force to overwrite)".to_string());
            }
            _ => {}
        }
    }

    // Merge team_config_url.
    if let Some(remote_tc) = &remote.global.team_config_url {
        if local.global.team_config_url.is_none() {
            changes.push(format!("Set team_config_url to {}", remote_tc));
            if !opts.dry_run {
                local.global.team_config_url = Some(remote_tc.clone());
            }
        }
    }

    // Merge language entries.
    for (lang, remote_cfg) in &remote.languages {
        if local.languages.contains_key(lang) {
            if opts.force {
                changes.push(format!(
                    "Update {}: {} → {}",
                    lang, local.languages[lang].url, remote_cfg.url
                ));
                if !opts.dry_run {
                    local.languages.insert(lang.clone(), remote_cfg.clone());
                }
            } else {
                changes.push(format!(
                    "Skip {} (already configured; use --force to overwrite)",
                    lang
                ));
            }
        } else {
            changes.push(format!("Add {}: {}", lang, remote_cfg.url));
            if !opts.dry_run {
                local.languages.insert(lang.clone(), remote_cfg.clone());
            }
        }
    }

    Ok(changes)
}
