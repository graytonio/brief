/// inject command — orchestrates detect + cache + output.
///
/// Called by the Claude Code SessionStart hook. Outputs assembled standards
/// to stdout. Always exits 0.
use std::env;
use std::path::PathBuf;

use crate::cache;
use crate::config::Config;
use crate::detect;

/// Runs the inject command: detect all matching entries, fetch/cache standards, print to stdout.
///
/// Returns Ok(()) always. Errors are handled gracefully (warn to stderr, continue).
pub fn run_inject() {
    let config = match Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("brief: warning: could not load config: {}", e);
            return;
        }
    };

    let ttl = config.global.cache_ttl;
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let mut blocks: Vec<String> = Vec::new();

    for detected in detect::detect_languages(&cwd, &config.languages) {
        if let Some(lang_cfg) = config.languages.get(&detected.language) {
            match cache::fetch_with_cache(&lang_cfg.url, ttl) {
                Ok(content) => blocks.push(content),
                Err(e) => eprintln!(
                    "brief: warning: could not load {} standards: {}",
                    detected.language, e
                ),
            }
        }
    }

    let output = blocks.join("\n---\n");
    if !output.is_empty() {
        print!("{}", output);
    }
}
