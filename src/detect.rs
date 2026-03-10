/// Project type detection by walking up the directory tree.
///
/// For each registered language, we look for the configured detection files
/// starting from the given directory and moving toward the filesystem root.
/// The first language whose detection file is found wins.
use std::path::{Path, PathBuf};

use crate::config::LanguageConfig;
use std::collections::HashMap;

/// Result of a successful language detection.
#[derive(Debug, Clone)]
pub struct DetectResult {
    /// The language name as registered in the config.
    pub language: String,
    /// The detection file that was found.
    pub file_found: String,
    /// The directory where the detection file was found.
    pub directory: PathBuf,
}

/// Walks up from `start` looking for any of the `detect` filenames.
/// Returns the first match found, or None.
fn find_file_upward(start: &Path, filenames: &[String]) -> Option<(String, PathBuf)> {
    let mut current = start.to_path_buf();
    loop {
        for name in filenames {
            let candidate = current.join(name);
            if candidate.exists() {
                return Some((name.clone(), current.clone()));
            }
        }
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => return None,
        }
    }
}

/// Detects the project language for the given directory using the registered language configs.
///
/// Languages are checked in alphabetical order (deterministic). The first match wins.
pub fn detect_language(
    cwd: &Path,
    languages: &HashMap<String, LanguageConfig>,
) -> Option<DetectResult> {
    // Sort for deterministic ordering.
    let mut entries: Vec<(&String, &LanguageConfig)> = languages.iter().collect();
    entries.sort_by_key(|(name, _)| name.as_str());

    for (language, lang_cfg) in entries {
        if lang_cfg.detect.is_empty() {
            continue;
        }
        if let Some((file_found, directory)) = find_file_upward(cwd, &lang_cfg.detect) {
            return Some(DetectResult {
                language: language.clone(),
                file_found,
                directory,
            });
        }
    }
    None
}
