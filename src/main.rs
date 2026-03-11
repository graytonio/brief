/// brief — Remote Standards Manager for Claude Code.
///
/// CLI entry point and command dispatch.
mod cache;
mod config;
mod detect;
mod hook;
mod inject;
mod sync;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::env;
use std::path::PathBuf;

// ─────────────────────────────────────────────────────────────────────────────
// CLI definitions
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(
    name = "brief",
    about = "Remote standards manager for Claude Code",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// First-time setup: create config and install Claude Code hook.
    Init {
        /// Immediately sync from this team config URL after init.
        #[arg(long, value_name = "URL")]
        team_config: Option<String>,
    },

    /// Register or replace the remote URL for a language.
    Add {
        /// Language name (e.g. rust, kotlin, python).
        language: String,
        /// Remote URL returning plain-text CLAUDE.md content.
        url: String,
        /// Override detection filenames (comma-separated).
        #[arg(long, value_name = "FILES")]
        detect: Option<String>,
    },

    /// Remove the URL registration for a language.
    Remove {
        /// Language name to remove.
        language: String,
    },

    /// List all registered languages, URLs, and cache status.
    List,

    /// Fetch a remote team config and merge it into the local config.
    Sync {
        /// Remote team config URL (uses team_config_url from config if omitted).
        url: Option<String>,
        /// Overwrite existing local entries with remote values.
        #[arg(long)]
        force: bool,
        /// Print what would change without writing to disk.
        #[arg(long)]
        dry_run: bool,
    },

    /// Invalidate cache and re-fetch standards. Omit language to re-fetch all.
    Update {
        /// Language to update (omit for all).
        language: Option<String>,
    },

    /// Print a summary of what would be injected in the current directory.
    Status,

    /// Internal: output assembled standards to stdout (called by Claude Code hook).
    Inject,

    /// Manage the Claude Code SessionStart hook.
    Hook {
        #[command(subcommand)]
        action: HookAction,
    },
}

#[derive(Subcommand)]
enum HookAction {
    /// Install (or reinstall) the SessionStart hook.
    Install,
    /// Remove the SessionStart hook entry.
    Uninstall,
}

// ─────────────────────────────────────────────────────────────────────────────
// main
// ─────────────────────────────────────────────────────────────────────────────

fn main() {
    let cli = Cli::parse();
    let result = dispatch(cli.command);
    if let Err(e) = result {
        eprintln!("brief: error: {:#}", e);
        std::process::exit(1);
    }
}

fn dispatch(command: Commands) -> Result<()> {
    match command {
        Commands::Init { team_config } => cmd_init(team_config),
        Commands::Add {
            language,
            url,
            detect,
        } => cmd_add(language, url, detect),
        Commands::Remove { language } => cmd_remove(language),
        Commands::List => cmd_list(),
        Commands::Sync {
            url,
            force,
            dry_run,
        } => cmd_sync(url, force, dry_run),
        Commands::Update { language } => cmd_update(language),
        Commands::Status => cmd_status(),
        Commands::Inject => {
            inject::run_inject();
            Ok(())
        }
        Commands::Hook { action } => match action {
            HookAction::Install => hook::install_hook(),
            HookAction::Uninstall => hook::uninstall_hook(),
        },
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// init
// ─────────────────────────────────────────────────────────────────────────────

fn cmd_init(team_config_url: Option<String>) -> Result<()> {
    use std::fs;

    let brief_dir = config::brief_dir()?;
    fs::create_dir_all(&brief_dir)
        .with_context(|| format!("Failed to create {}", brief_dir.display()))?;

    let config_path = config::config_path()?;
    if !config_path.exists() {
        let cfg = config::Config::default();
        cfg.save()?;
        println!("Created {}", config_path.display());
    } else {
        println!("Config already exists at {}", config_path.display());
    }

    // Install Claude Code hook.
    hook::install_hook()?;

    // Warn if brief is not on PATH.
    if which_brief().is_none() {
        eprintln!(
            "brief: warning: 'brief' was not found on $PATH. \
             Make sure the binary is installed to a location on your PATH \
             so the Claude Code hook can invoke it."
        );
    }

    // Optionally sync from a team config URL.
    if let Some(url) = team_config_url {
        println!("Syncing from team config: {}", url);
        let mut cfg = config::Config::load()?;
        let opts = sync::SyncOptions {
            force: false,
            dry_run: false,
        };
        let changes = sync::run_sync(&url, &mut cfg, &opts)?;
        cfg.global.team_config_url = Some(url);
        cfg.save()?;
        for change in &changes {
            println!("  {}", change);
        }
        if changes.is_empty() {
            println!("  No changes.");
        }
    }

    println!();
    println!("Next steps:");
    println!("  brief add rust https://example.com/rust/CLAUDE.md");
    println!("  brief add python https://example.com/python/CLAUDE.md");
    println!("  brief list");

    Ok(())
}

/// Returns the path to the `brief` binary if it is on $PATH.
fn which_brief() -> Option<PathBuf> {
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths).find_map(|dir| {
            let candidate = dir.join("brief");
            if candidate.is_file() {
                Some(candidate)
            } else {
                None
            }
        })
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// add
// ─────────────────────────────────────────────────────────────────────────────

fn cmd_add(language: String, url: String, detect_arg: Option<String>) -> Result<()> {
    let mut cfg = config::Config::load()?;

    let detect_files: Vec<String> = if let Some(raw) = detect_arg {
        raw.split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        config::default_detect_files(&language)
        // Empty vec (no defaults, no --detect) means this entry matches every session.
    };

    cfg.languages.insert(
        language.clone(),
        config::LanguageConfig {
            url: url.clone(),
            detect: detect_files,
        },
    );

    cfg.save()?;
    println!("Added {}: {}", language, url);
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// remove
// ─────────────────────────────────────────────────────────────────────────────

fn cmd_remove(language: String) -> Result<()> {
    let mut cfg = config::Config::load()?;
    if cfg.languages.remove(&language).is_some() {
        cfg.save()?;
        println!("Removed '{}'.", language);
    } else {
        eprintln!("brief: '{}' is not registered.", language);
        std::process::exit(1);
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// list
// ─────────────────────────────────────────────────────────────────────────────

fn cmd_list() -> Result<()> {
    let cfg = config::Config::load()?;

    // Collect rows: (language, url, cached, last_fetch_display)
    let mut rows: Vec<(String, String, bool, String)> = Vec::new();

    let mut lang_names: Vec<&String> = cfg.languages.keys().collect();
    lang_names.sort();
    for lang in lang_names {
        let lc = &cfg.languages[lang];
        let (cached, last_fetch) = cache_display(&lc.url);
        rows.push((lang.clone(), lc.url.clone(), cached, last_fetch));
    }

    if rows.is_empty() {
        println!("No languages registered. Run 'brief add <language> <url>' to get started.");
        return Ok(());
    }

    // Column widths.
    let lang_w = rows.iter().map(|r| r.0.len()).max().unwrap_or(8).max(8);
    let url_w = rows.iter().map(|r| r.1.len()).max().unwrap_or(50).max(50);

    println!(
        " {:<lang_w$}  {:<url_w$}  {:<7}  Last Fetch",
        "Language",
        "URL",
        "Cached",
        lang_w = lang_w,
        url_w = url_w
    );
    println!(
        " {}  {}  {}  {}",
        "─".repeat(lang_w),
        "─".repeat(url_w),
        "─".repeat(7),
        "─".repeat(18)
    );
    for (lang, url, cached, last_fetch) in &rows {
        println!(
            " {:<lang_w$}  {:<url_w$}  {:<7}  {}",
            lang,
            url,
            if *cached { "✓" } else { "✗" },
            last_fetch,
            lang_w = lang_w,
            url_w = url_w
        );
    }

    Ok(())
}

/// Returns (is_cached, last_fetch_string) for display.
fn cache_display(url: &str) -> (bool, String) {
    match cache::cached_timestamp(url) {
        Some(ts) => {
            let cached = cache::read_cache(url).is_some();
            let dt = format_timestamp(ts);
            (cached, dt)
        }
        None => (false, "never".into()),
    }
}

/// Formats a Unix timestamp as a human-readable date string.
fn format_timestamp(ts: u64) -> String {
    // Use chrono for nice formatting.
    let secs = ts;
    match chrono::DateTime::from_timestamp(secs as i64, 0) {
        Some(dt) => dt.format("%Y-%m-%d %H:%M").to_string(),
        None => format!("{}", secs),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// sync
// ─────────────────────────────────────────────────────────────────────────────

fn cmd_sync(url_arg: Option<String>, force: bool, dry_run: bool) -> Result<()> {
    let mut cfg = config::Config::load()?;

    let url = url_arg
        .or_else(|| cfg.global.team_config_url.clone())
        .context("No URL provided and no team_config_url in config. Run: brief sync <url>")?;

    println!("Syncing from: {}", url);
    if dry_run {
        println!("(dry run — no changes will be written)");
    }

    let opts = sync::SyncOptions { force, dry_run };
    let changes = sync::run_sync(&url, &mut cfg, &opts)?;

    for change in &changes {
        println!("  {}", change);
    }
    if changes.is_empty() {
        println!("  No changes.");
    }

    if !dry_run {
        cfg.save()?;
        println!("Config saved.");
    }

    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// update
// ─────────────────────────────────────────────────────────────────────────────

fn cmd_update(language: Option<String>) -> Result<()> {
    let cfg = config::Config::load()?;
    let ttl = cfg.global.cache_ttl;

    match language {
        Some(lang) => {
            let lc = cfg
                .languages
                .get(&lang)
                .with_context(|| format!("'{}' is not registered", lang))?;
            update_one(&lang, &lc.url, ttl)?;
        }
        None => {
            // Re-fetch all.
            let mut langs: Vec<&String> = cfg.languages.keys().collect();
            langs.sort();
            for lang in langs {
                let lc = &cfg.languages[lang];
                update_one(lang, &lc.url, ttl)?;
            }
        }
    }

    Ok(())
}

fn update_one(label: &str, url: &str, _ttl: u64) -> Result<()> {
    print!("Updating {} ... ", label);
    cache::invalidate_cache(url)?;
    match cache::fetch_url(url) {
        Ok(content) => {
            cache::write_cache(url, &content)?;
            println!("ok");
        }
        Err(e) => {
            println!("FAILED: {}", e);
        }
    }
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// status
// ─────────────────────────────────────────────────────────────────────────────

fn cmd_status() -> Result<()> {
    let cfg = config::Config::load()?;
    let cwd = env::current_dir().context("Failed to get current directory")?;

    println!("Current directory: {}", cwd.display());

    let matches = detect::detect_languages(&cwd, &cfg.languages);

    println!();
    println!("Injection order (later = higher priority):");

    if matches.is_empty() {
        println!("  (nothing — no matching standards configured)");
    } else {
        for (idx, d) in matches.iter().enumerate() {
            if let Some(lc) = cfg.languages.get(&d.language) {
                let age = cache_age_display(&lc.url);
                let cached_note = if cache::read_cache(&lc.url).is_some() {
                    format!("cached, {}", age)
                } else {
                    "not cached".into()
                };
                let match_note = if d.file_found.is_empty() {
                    "(always)".into()
                } else {
                    format!("(found {} at {})", d.file_found, d.directory.display())
                };
                println!(
                    "  [{}] {}  {}  {}  {}",
                    idx + 1,
                    d.language,
                    lc.url,
                    cached_note,
                    match_note
                );
            }
        }
    }

    // Hook status.
    println!();
    let settings = hook::read_settings().unwrap_or(serde_json::json!({}));
    let hook_status = if hook::is_hook_installed(&settings) {
        format!(
            "installed (SessionStart in {})",
            hook::settings_path()
                .map(|p| p.display().to_string())
                .unwrap_or_default()
        )
    } else {
        "not installed  (run: brief hook install)".into()
    };
    println!("Hook status: {}", hook_status);

    Ok(())
}

fn cache_age_display(url: &str) -> String {
    match cache::cache_age_secs(url) {
        Some(secs) if secs < 60 => format!("{} sec old", secs),
        Some(secs) if secs < 3600 => format!("{} min old", secs / 60),
        Some(secs) => format!("{} hr old", secs / 3600),
        None => "never fetched".into(),
    }
}
