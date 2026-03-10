/// Claude Code settings.json read/merge/write for hook management.
///
/// The hook JSON structure we install:
/// {
///   "hooks": {
///     "SessionStart": [
///       {
///         "hooks": [
///           {
///             "type": "command",
///             "command": "brief inject",
///             "statusMessage": "Loading team standards...",
///             "timeout": 10
///           }
///         ]
///       }
///     ]
///   }
/// }
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

/// Returns the path to ~/.claude/settings.json.
pub fn settings_path() -> Result<PathBuf> {
    let home = dirs::home_dir().context("Could not determine home directory")?;
    Ok(home.join(".claude").join("settings.json"))
}

/// The unique marker used to identify our hook entry.
const HOOK_COMMAND: &str = "brief inject";

/// Reads the current settings.json, or returns an empty object if it doesn't exist.
pub fn read_settings() -> Result<Value> {
    let path = settings_path()?;
    if !path.exists() {
        return Ok(json!({}));
    }
    let contents =
        fs::read_to_string(&path).with_context(|| format!("Failed to read {}", path.display()))?;
    if contents.trim().is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str(&contents).with_context(|| format!("Failed to parse {}", path.display()))
}

/// Writes the settings value back to settings.json, creating parent dirs as needed.
pub fn write_settings(settings: &Value) -> Result<()> {
    let path = settings_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }
    let contents =
        serde_json::to_string_pretty(settings).context("Failed to serialise settings.json")?;
    fs::write(&path, contents).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

/// Builds our hook entry object.
fn brief_hook_entry() -> Value {
    json!({
        "hooks": [
            {
                "type": "command",
                "command": HOOK_COMMAND,
                "statusMessage": "Loading team standards...",
                "timeout": 10
            }
        ]
    })
}

/// Returns true if the settings already contain our hook entry.
pub fn is_hook_installed(settings: &Value) -> bool {
    if let Some(session_start) = settings
        .get("hooks")
        .and_then(|h| h.get("SessionStart"))
        .and_then(|v| v.as_array())
    {
        for entry in session_start {
            if let Some(hooks) = entry.get("hooks").and_then(|h| h.as_array()) {
                for hook in hooks {
                    if hook.get("command").and_then(|c| c.as_str()) == Some(HOOK_COMMAND) {
                        return true;
                    }
                }
            }
        }
    }
    false
}

/// Installs the brief hook into settings.json, merging non-destructively.
/// If the hook is already present, this is a no-op.
pub fn install_hook() -> Result<()> {
    let mut settings = read_settings()?;

    if is_hook_installed(&settings) {
        println!("Hook already installed.");
        return Ok(());
    }

    // Ensure hooks.SessionStart is an array.
    let session_start = settings
        .pointer_mut("/hooks/SessionStart")
        .and_then(|v| v.as_array_mut());

    if session_start.is_some() {
        // Append to the existing array.
        settings["hooks"]["SessionStart"]
            .as_array_mut()
            .unwrap()
            .push(brief_hook_entry());
    } else {
        // Create the hooks.SessionStart path if needed.
        if settings.get("hooks").is_none() {
            settings["hooks"] = json!({});
        }
        settings["hooks"]["SessionStart"] = json!([brief_hook_entry()]);
    }

    write_settings(&settings)?;
    println!("Hook installed into {}", settings_path()?.display());
    Ok(())
}

/// Removes the brief hook entry from settings.json.
/// Other hooks and settings are left untouched.
pub fn uninstall_hook() -> Result<()> {
    let mut settings = read_settings()?;

    if !is_hook_installed(&settings) {
        println!("Hook is not currently installed.");
        return Ok(());
    }

    if let Some(session_start) = settings
        .pointer_mut("/hooks/SessionStart")
        .and_then(|v| v.as_array_mut())
    {
        session_start.retain(|entry| {
            if let Some(hooks) = entry.get("hooks").and_then(|h| h.as_array()) {
                for hook in hooks {
                    if hook.get("command").and_then(|c| c.as_str()) == Some(HOOK_COMMAND) {
                        return false;
                    }
                }
            }
            true
        });
    }

    write_settings(&settings)?;
    println!("Hook removed from {}", settings_path()?.display());
    Ok(())
}
