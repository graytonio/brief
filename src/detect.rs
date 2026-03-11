/// Project type detection by walking up the directory tree.
///
/// For each registered entry, we check if its detection files are found
/// starting from the given directory and moving toward the filesystem root.
/// Entries with no detection files match every session (always loaded).
/// All matching entries are returned, in alphabetical order.
use std::path::{Path, PathBuf};

use crate::config::LanguageConfig;
use std::collections::HashMap;

/// Result of a successful language detection.
#[derive(Debug, Clone)]
pub struct DetectResult {
    /// The language name as registered in the config.
    pub language: String,
    /// The detection file that was found, or empty string for always-match entries.
    pub file_found: String,
    /// The directory where the detection file was found, or cwd for always-match entries.
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

/// Returns all matching entries for the given directory.
///
/// Entries are checked in alphabetical order (deterministic).
/// Entries with an empty `detect` list always match.
/// Entries with detection files match if any file is found walking up from cwd.
pub fn detect_languages(
    cwd: &Path,
    languages: &HashMap<String, LanguageConfig>,
) -> Vec<DetectResult> {
    let mut entries: Vec<(&String, &LanguageConfig)> = languages.iter().collect();
    entries.sort_by_key(|(name, _)| name.as_str());

    let mut results = Vec::new();
    for (language, lang_cfg) in entries {
        if lang_cfg.detect.is_empty() {
            results.push(DetectResult {
                language: language.clone(),
                file_found: String::new(),
                directory: cwd.to_path_buf(),
            });
        } else if let Some((file_found, directory)) = find_file_upward(cwd, &lang_cfg.detect) {
            results.push(DetectResult {
                language: language.clone(),
                file_found,
                directory,
            });
        }
    }
    results
}
