# brief

**Remote standards manager for Claude Code.**

`brief` lets teams centrally host CLAUDE.md coding standards files and automatically inject the correct ones into every Claude Code session based on the project's language. Register a URL once; every developer on the team picks up updates within the configured TTL with no manual action.

---

## How it works

When a Claude Code session starts, `brief inject` is called by a hook you install once. It:

1. Detects the project type by walking up the directory tree looking for marker files (`Cargo.toml`, `package.json`, `go.mod`, etc.)
2. Fetches (or uses a cached copy of) the standards for that language
3. Outputs the assembled content to stdout, which Claude Code injects as session context

Standards are loaded in two layers:

| Priority | Layer | Description |
|----------|-------|-------------|
| 1 — lowest | **global** | Always injected. Baseline rules for every session. |
| 2 — highest | **language-specific** | Injected after global. Overrides any conflicting global rules. |

---

## Installation

### Homebrew (macOS and Linux)

```sh
brew tap graytonw/brief
brew install brief
```

### Build from source

```sh
cargo install --path .
```

Or build manually:

```sh
cargo build --release
cp target/release/brief /usr/local/bin/brief
```

### First-time setup

```sh
brief init
```

This creates `~/.brief/config.toml` and installs the Claude Code `SessionStart` hook in `~/.claude/settings.json`.

**Onboarding from a team config URL:**

```sh
brief init --team-config https://raw.githubusercontent.com/ORG/REPO/main/.brief.toml
```

This runs `init` and then immediately syncs all language URLs from the team config.

---

## Usage

### Register a language URL

```sh
brief add rust https://raw.githubusercontent.com/ORG/REPO/main/rust/CLAUDE.md
brief add kotlin https://raw.githubusercontent.com/ORG/REPO/main/kotlin/CLAUDE.md
```

For languages without built-in detection rules, specify the marker files:

```sh
brief add scala https://raw.githubusercontent.com/ORG/REPO/main/scala/CLAUDE.md \
  --detect build.sbt
```

**Built-in detection defaults:**

| Language | Marker files |
|----------|-------------|
| rust | `Cargo.toml` |
| kotlin | `build.gradle.kts`, `build.gradle` |
| python | `pyproject.toml`, `setup.py` |
| javascript | `package.json` |
| typescript | `package.json` |
| go | `go.mod` |

### Set a global baseline

```sh
brief set-global https://raw.githubusercontent.com/ORG/REPO/main/CLAUDE.md
```

This URL is injected into every session regardless of language.

### Sync from a team config

```sh
# Use team_config_url from ~/.brief/config.toml
brief sync

# Or specify a URL directly
brief sync https://raw.githubusercontent.com/ORG/REPO/main/.brief.toml

# Preview what would change without writing
brief sync --dry-run

# Overwrite local entries with remote values
brief sync --force
```

### Check current status

```sh
brief status
```

```
Current directory: /Users/grayton/projects/my-service
Detected language: rust  (found Cargo.toml at /Users/grayton/projects/my-service)

Injection order (later = higher priority):
  [1] global   https://.../CLAUDE.md             (cached, 23 min old)  ← baseline
  [2] rust     https://.../rust/CLAUDE.md         (cached, 23 min old)  ← overrides [1]

Hook status: installed (SessionStart in ~/.claude/settings.json)
```

### List all registered URLs

```sh
brief list
```

```
 Language  URL                                                   Cached   Last Fetch
 ────────  ────────────────────────────────────────────────────  ───────  ──────────────────
 global    https://.../CLAUDE.md                                 ✓        2026-03-10 09:14
 rust      https://.../rust/CLAUDE.md                            ✓        2026-03-10 09:14
 kotlin    https://.../kotlin/CLAUDE.md                          ✗        never
```

### Force a refresh

```sh
# Re-fetch all registered URLs
brief update

# Re-fetch only one language
brief update rust
```

### Remove a language

```sh
brief remove kotlin
```

### Manage the hook manually

```sh
# Reinstall after a Claude Code update
brief hook install

# Remove the hook
brief hook uninstall
```

---

## Team setup (platform engineers)

Publish a `.brief.toml` file to a URL your team can reach. The format mirrors `~/.brief/config.toml`:

```toml
[global]
url = "https://raw.githubusercontent.com/ORG/REPO/main/CLAUDE.md"
cache_ttl = 3600

[languages.rust]
url    = "https://raw.githubusercontent.com/ORG/REPO/main/rust/CLAUDE.md"
detect = ["Cargo.toml"]

[languages.python]
url    = "https://raw.githubusercontent.com/ORG/REPO/main/python/CLAUDE.md"
detect = ["pyproject.toml", "setup.py"]
```

Then give your team a one-liner:

```sh
brief init --team-config https://raw.githubusercontent.com/ORG/REPO/main/.brief.toml
```

When you update a standards file, all developers pick it up automatically within the configured `cache_ttl` (default: 1 hour).

---

## Configuration

Config file: `~/.brief/config.toml`

```toml
[global]
url            = "https://..."   # Global baseline URL
cache_ttl      = 3600            # Cache TTL in seconds (default: 3600)
team_config_url = "https://..."  # URL for 'brief sync' with no arguments

[languages.rust]
url    = "https://..."
detect = ["Cargo.toml"]
```

### Authentication

For private repositories, provide an auth token via:

1. Environment variable: `CLAUDE_STANDARDS_TOKEN`
2. File: `~/.brief/.token` (should be mode `600`)

The token is sent as `Authorization: Bearer <token>` on all requests.

---

## Caching

Fetched content is stored in `~/.brief/cache/`. If a network request fails, `brief` falls back to the last cached version automatically and warns on stderr. This means Claude Code always starts even when offline.

---

## Uninstalling

```sh
brief hook uninstall
rm -rf ~/.brief
```
