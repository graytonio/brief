# PRD: `brief` — Remote Standards Manager for Claude Code

**Status:** Draft
**Author:** Grayton Ward
**Last Updated:** 2026-03-10

---

## Overview

`brief` is a CLI tool that lets teams centrally host and manage CLAUDE.md
coding standards files and automatically inject the correct ones into Claude Code sessions
based on the project's language. Users register a remote URL per language or project type;
the tool handles detection, caching, and hook integration so the agent always works to
the latest team standards without any per-developer maintenance.

**Why "brief"?** The name comes from the legal and military concept of a briefing —
delivering the precise context and rules someone needs before they act. Every time a
Claude Code session starts, `brief` does exactly that: it fetches the relevant standards
and briefs the agent before the first prompt is ever written. The name is short, verb-
and noun-ambiguous (you *brief* the agent; the standards *are* the brief), and the
metaphor holds consistently across the entire CLI — `brief add`, `brief sync`,
`brief status` all read naturally.

Standards are loaded in two layers. A **global** URL acts as the baseline default and is
always injected. If the current project matches a registered language, that language's
standards are injected **after** the global layer, giving them higher priority. Language-
specific rules therefore always take precedence over global ones when the two conflict.

---

## Problem Statement

Teams using Claude Code face a standards drift problem: coding conventions, test
requirements, and agent guardrails are either duplicated across dozens of repos or
simply not present. Today's workaround (manually placing CLAUDE.md files and relying on
developers to keep them current) does not scale. There is no mechanism to push a
standards update to the whole team's agent context without each developer manually
pulling and replacing files.

---

## Goals

- Allow any developer to register a remote URL as the authoritative standards source
  for a given language or project type.
- Automatically detect the project type at session start and inject the correct standards
  as context for Claude Code.
- Load standards in a defined two-layer priority order: global standards first as a
  universal baseline, language-specific standards second at higher priority so they
  can extend or override anything in the global layer.
- Allow teams to host a single shared config file so that `brief sync` brings
  a fresh machine fully up to date in one command.
- Standards updates propagate to the entire team within a configurable TTL with no
  action required from individual developers.
- Work with any URL that returns plain text (GitHub raw, S3, internal CDN, etc.).

## Non-Goals

- This tool does not manage or version the content of the standards files themselves.
  That is the responsibility of the repo hosting them.
- This tool does not integrate with any specific CI/CD system.
- This tool does not support editors or IDEs other than Claude Code (though the hook
  mechanism is simple enough to adapt).
- This tool does not enforce standards compliance — it only ensures standards are present
  as context.

---

## User Personas

**The Platform Engineer** sets up the canonical standards repo, publishes a team config
URL, and distributes a one-liner onboarding command to the team. They need confidence
that every developer's agent is using the authoritative version of the standards.

**The Developer** runs one setup command and then never thinks about it again. The tool
loads the right standards automatically when they start a Claude Code session in any
project.

**The New Hire** runs the team onboarding command on their first day. Their agent
immediately works to the same standards as the rest of the team.

---

## User Stories

- As a platform engineer, I can publish a team config TOML to a URL and have all
  developers sync from it with one command, so I have a single source of truth.
- As a developer, I can register a remote URL for Rust and another for Kotlin, so my
  Claude Code agent loads the correct standards automatically in each project.
- As a developer, I can run `brief status` to see which standards will be
  loaded in my current directory and when the cache was last refreshed.
- As a developer, I can run `brief update` to force-pull the latest standards
  without waiting for the TTL to expire.
- As a platform engineer, I can update the remote CLAUDE.md file and know it will be
  picked up by all developers' agents within the configured TTL (default: 1 hour).
- As a developer, I can define custom detection rules so that a project root containing
  `pyproject.toml` loads Python standards, even if Python is not a first-class language.
- As a developer, the tool should degrade gracefully when offline, falling back to the
  last cached version of each standards file.

---

## Functional Requirements

### Configuration

The tool maintains a single config file at `~/.brief/config.toml`. This file
maps language identifiers to remote URLs and specifies detection rules for each.

**Config schema:**

```toml
# Global standards — the baseline injected into EVERY session regardless of language.
# Loaded first (lowest priority layer). Language-specific standards are always
# injected after this block and take precedence over anything defined here.
[global]
url = "https://raw.githubusercontent.com/ORG/REPO/main/CLAUDE.md"

# How long (seconds) to use a cached copy before re-fetching. Default: 3600.
cache_ttl = 3600

# If set, 'brief sync' pulls and merges this remote config.
# This is the URL a platform team publishes once for everyone to point at.
team_config_url = "https://raw.githubusercontent.com/ORG/REPO/main/.brief.toml"

# Language-specific standards — higher priority than [global].
# Each entry specifies:
#   url     - remote URL returning plain-text CLAUDE.md content
#   detect  - one or more filenames; if any are found walking up from CWD, this
#             language's standards are loaded after the global block. First match wins.
# Rules in a language block override conflicting rules in [global].
[languages.rust]
url    = "https://raw.githubusercontent.com/ORG/REPO/main/rust/CLAUDE.md"
detect = ["Cargo.toml"]

[languages.kotlin]
url    = "https://raw.githubusercontent.com/ORG/REPO/main/kotlin/CLAUDE.md"
detect = ["build.gradle.kts", "build.gradle"]
```

Config values set by `sync` are marked with a `# synced` comment and can be overridden
locally. Local overrides persist across syncs.

### Standards Loading Priority

This is a core invariant of the tool and must be preserved in all commands and in `inject`.

Standards are always assembled and injected in the following fixed order:

```
Priority 1 (lowest) — global
  The content of [global].url, if configured.
  Injected first. Forms the baseline for every session.

Priority 2 (highest) — language-specific
  The content of [languages.<match>].url, if a detection rule matches the CWD.
  Injected second, immediately after the global block.
  Any rule or instruction in this block supersedes a conflicting rule in the global block,
  because Claude Code treats later context as higher priority.
```

If no language match is found, only the global block is injected. If no global URL is
configured either, `inject` outputs nothing and Claude Code starts with no injected
standards.

**There is no mechanism for injecting more than two blocks.** If a project matches
multiple languages (which the detection algorithm prevents by design — first match wins),
only the first-matched language is loaded.

The `status` command must display standards in this injection order with explicit priority
labels so developers can reason about what the agent will see.

### CLI Commands

#### `brief init`

Performs first-time setup:
1. Creates `~/.brief/` and an empty `config.toml`.
2. Installs the Claude Code `SessionStart` hook into `~/.claude/settings.json`,
   merging non-destructively with any existing hook config.
3. Prints next steps (register a language URL, or sync from a team config URL).

Flags:
- `--team-config <url>` — runs `sync` immediately after init.

#### `brief add <language> <url>`

Registers or replaces the remote URL for `<language>`.

```
brief add rust https://raw.githubusercontent.com/ORG/REPO/main/rust/CLAUDE.md
brief add kotlin https://raw.githubusercontent.com/ORG/REPO/main/kotlin/CLAUDE.md
```

Flags:
- `--detect <file>[,<file>...]` — override the default detection filenames for this
  language. Defaults: rust → `Cargo.toml`, kotlin → `build.gradle.kts,build.gradle`.
  For other languages, `--detect` is required.

#### `brief remove <language>`

Removes the URL registration for `<language>`. Does not affect cached files.

#### `brief set-global <url>`

Sets the global standards URL applied to all sessions regardless of language.

#### `brief list`

Prints all registered languages and their URLs in a table, including the global entry if
set. Indicates whether a valid cache exists for each entry and when it was last fetched.

```
 Language  URL                                                   Cached   Last Fetch
 ────────  ────────────────────────────────────────────────────  ───────  ──────────────────
 global    https://.../CLAUDE.md                                 ✓        2026-03-10 09:14
 rust      https://.../rust/CLAUDE.md                            ✓        2026-03-10 09:14
 kotlin    https://.../kotlin/CLAUDE.md                          ✗        never
```

#### `brief sync [<url>]`

Fetches a remote team config TOML and merges it into the local config. Uses
`team_config_url` from the local config if no argument is given.

Merge rules:
- Remote entries for languages not present locally are added.
- Remote entries for languages already locally configured are skipped unless
  `--force` is passed.
- The `global` URL is updated only if `--force` is passed.

Flags:
- `--force` — overwrite all local entries with remote values.
- `--dry-run` — print what would change without writing to disk.

#### `brief update [<language>]`

Invalidates the cache for one or all languages and immediately re-fetches from the
configured URLs. Useful after a standards file has been updated remotely.

- `brief update` — re-fetches all registered URLs.
- `brief update rust` — re-fetches only the Rust standards.

#### `brief status`

Prints a summary of what would be injected into Claude Code if started in the current
working directory:

```
Current directory: /Users/grayton/projects/my-service
Detected language: rust  (found Cargo.toml at /Users/grayton/projects/my-service)

Injection order (later = higher priority):
  [1] global   https://.../CLAUDE.md             (cached, 23 min old)  ← baseline
  [2] rust     https://.../rust/CLAUDE.md         (cached, 23 min old)  ← overrides [1]

Hook status: installed (SessionStart in ~/.claude/settings.json)
```

#### `brief inject`

**Internal command called by the Claude Code hook — not intended for direct use.**

Outputs to stdout the assembled standards content, which Claude Code injects as session
context. Assembly follows the fixed priority order defined in the Standards Loading
Priority section:

1. Global standards block is written to stdout first (baseline, lowest priority).
2. A separator line (`---`) is written between blocks.
3. Language-specific block is written to stdout second, immediately after the separator
   (highest priority). Because Claude Code treats content appearing later in the context
   as higher priority, language-specific rules will override any conflicting global rules.

If no language match is found, only step 1 is performed. If no global URL is configured,
only step 3 is performed (no separator). If neither is configured, `inject` outputs
nothing and exits 0.

Handles caching and TTL automatically. Falls back to stale cache if the network is
unavailable. Never exits non-zero so a network failure never blocks Claude Code from
starting.

#### `brief hook install`

(Re)installs the Claude Code `SessionStart` hook. Useful after a Claude Code update that
may have reset settings.

#### `brief hook uninstall`

Removes the tool's hook entry from `~/.claude/settings.json`. Does not delete the local
config or cache.

### Caching

- All fetched content is stored in `~/.brief/cache/` as plain text files
  named by a hash of the URL.
- A companion timestamp file records the time of last successful fetch.
- If a fetch fails and a cache file exists (regardless of age), the cache is used and
  a warning is written to stderr.
- If a fetch fails and no cache exists, `inject` outputs nothing for that entry
  (rather than erroring), ensuring Claude Code still starts.

### Hook Integration

The installed hook entry in `~/.claude/settings.json` is:

```json
{
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "brief inject",
            "statusMessage": "Loading team standards...",
            "timeout": 10
          }
        ]
      }
    ]
  }
}
```

The tool must be on `$PATH` for this to work. The `init` command warns the user if it
is not.

### Authentication

For private repositories, the tool reads an optional auth token from:
1. The environment variable `CLAUDE_STANDARDS_TOKEN`.
2. The file `~/.brief/.token` (mode `600`).

The token is sent as `Authorization: Bearer <token>` on all requests. If neither is set,
requests are made unauthenticated.

---

## Technical Architecture

### Language

The tool should be implemented in **Rust** as a single statically-linked binary. This
makes distribution simple (copy one file), avoids runtime dependencies, and produces fast
startup suitable for a hook that runs on every session.

### Project Layout

```
brief/
├── src/
│   ├── main.rs           # CLI entry point, command dispatch
│   ├── config.rs         # Config file parsing and writing (TOML)
│   ├── cache.rs          # Fetch-with-cache logic, TTL handling
│   ├── detect.rs         # Project type detection (walk up directory tree)
│   ├── hook.rs           # Claude Code settings.json read/merge/write
│   ├── inject.rs         # inject command — orchestrates detect + cache + output
│   └── sync.rs           # sync command — remote config fetch and merge
├── tests/
│   └── integration/      # End-to-end tests against local HTTP fixtures
├── Cargo.toml
└── CLAUDE.md             # Standards for this project (dogfood)
```

### Key Dependencies (Cargo)

| Crate | Purpose |
|-------|---------|
| `clap` | CLI argument parsing |
| `toml` | Config file serialisation |
| `serde` / `serde_json` | JSON for Claude Code settings.json |
| `reqwest` (blocking) | HTTP fetching |
| `dirs` | Cross-platform home directory resolution |

### Config File Location

`~/.brief/config.toml` — created by `init`. On Windows, `%APPDATA%\brief\config.toml`.

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error (config not found, bad argument, etc.) |
| 2 | Network error with no cache fallback available |

`inject` always exits 0 unless a programming error occurs, so a network failure never
blocks Claude Code from starting.

---

## Non-Functional Requirements

- **Startup time**: `inject` must complete in under 200ms on a warm cache (no network
  call). Network fetches are acceptable on a cold or expired cache.
- **Offline resilience**: Any command that reads standards must function without network
  access, using stale cache if necessary.
- **Non-destructive config merges**: `init` and `hook install` must never overwrite
  existing `~/.claude/settings.json` entries unrelated to this tool.
- **Cross-platform**: macOS and Linux are required targets. Windows is a stretch goal.
- **No root required**: All files live in the user's home directory.

---

## Success Metrics

- A new developer can go from zero to a fully configured standards-aware Claude Code
  session in under 2 minutes using `init --team-config <url>`.
- After a platform engineer updates a remote standards file, all developers pick up the
  change within the configured TTL without any manual action.
- `inject` adds no perceptible latency to Claude Code session start on a warm cache.

---

## Out of Scope / Future Work

- **Project-level config** (`.brief.toml` in a repo) that overrides or extends
  the user-level config for that specific project.
- **Multiple URLs per language** — for teams that compose standards from several files.
- **Version pinning** — locking a project to a specific commit or tag of the remote
  standards file rather than always using `main`.
- **GUI / TUI** for browsing and editing registered URLs.
- **Homebrew / package manager distribution.**
