# brief

**Remote standards manager for Claude Code.**

`brief` lets teams centrally host standards files (`CLAUDE.md`, `AGENT.md`, or any markdown) and automatically inject the correct ones into every Claude Code session based on the project's language. Register a URL once; every developer on the team picks up updates within the configured TTL with no manual action.

Because `brief` layers multiple files in order, it's a natural fit for **composable agent standards**: a shared baseline, cross-cutting concerns (testing, security, API conventions), and language-specific rules all assembled automatically per project.

---

## How it works

When a Claude Code session starts, `brief inject` is called by a hook you install once. It:

1. Detects the project type by walking up the directory tree looking for marker files (`Cargo.toml`, `package.json`, `go.mod`, etc.)
2. Fetches (or uses a cached copy of) the standards for that language
3. Outputs the assembled content to stdout, which Claude Code injects as session context

Multiple entries can match a single session and are injected in order. Entries without `detect` files match every session and act as a baseline; entries with `detect` files are injected after, taking higher priority.

---

## Composable agent standards

`brief` assembles standards from multiple files in order. This lets you split concerns across focused files instead of maintaining one monolithic document.

### Recommended file naming

Use kebab-case names that describe the concern, not the technology:

```
standards/
├── AGENT.md                  # Global baseline — coding principles, commit style, review etiquette
├── AGENT.testing.md          # Testing standards — unit vs integration, snapshot rules, coverage
├── AGENT.security.md         # Security guidelines — OWASP top 10, secrets handling, auth patterns
├── AGENT.api-conventions.md  # API design rules — REST/GraphQL naming, pagination, error shapes
├── rust/
│   └── AGENT.md              # Rust-specific — Clippy config, error handling, unsafe policy
├── python/
│   └── AGENT.md              # Python-specific — type annotations, dependency management
└── typescript/
    └── AGENT.md              # TS-specific — strict mode, import style, framework conventions
```

### Register each concern separately

Files with no `--detect` flag are injected into every session (baseline + cross-cutting concerns). Language files are added on top when their marker file is found.

```sh
brief add baseline     https://raw.githubusercontent.com/ORG/REPO/main/standards/AGENT.md
brief add testing      https://raw.githubusercontent.com/ORG/REPO/main/standards/AGENT.testing.md
brief add security     https://raw.githubusercontent.com/ORG/REPO/main/standards/AGENT.security.md
brief add rust         https://raw.githubusercontent.com/ORG/REPO/main/standards/rust/AGENT.md
brief add typescript   https://raw.githubusercontent.com/ORG/REPO/main/standards/typescript/AGENT.md
```

For a Rust project, Claude Code receives (in order): `AGENT.md` → `AGENT.testing.md` → `AGENT.security.md` → `rust/AGENT.md`. Later files take higher priority, so language-specific rules can override shared defaults.

### Team `.brief.toml` for composable standards

```toml
[global]
cache_ttl = 3600

# Cross-cutting — always injected, in order
[languages.baseline]
url    = "https://raw.githubusercontent.com/ORG/REPO/main/standards/AGENT.md"
detect = []

[languages.testing]
url    = "https://raw.githubusercontent.com/ORG/REPO/main/standards/AGENT.testing.md"
detect = []

[languages.security]
url    = "https://raw.githubusercontent.com/ORG/REPO/main/standards/AGENT.security.md"
detect = []

# Language-specific — injected only when the marker file is found
[languages.rust]
url    = "https://raw.githubusercontent.com/ORG/REPO/main/standards/rust/AGENT.md"
detect = ["Cargo.toml"]

[languages.typescript]
url    = "https://raw.githubusercontent.com/ORG/REPO/main/standards/typescript/AGENT.md"
detect = ["package.json"]

[languages.python]
url    = "https://raw.githubusercontent.com/ORG/REPO/main/standards/python/AGENT.md"
detect = ["pyproject.toml", "setup.py"]
```

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

### Add a baseline (always-injected) entry

Register an entry with no `--detect` files and it will be injected into every session:

```sh
brief add baseline https://raw.githubusercontent.com/ORG/REPO/main/CLAUDE.md
```

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

Injection order (later = higher priority):
  [1] baseline  https://.../CLAUDE.md             (cached, 23 min old)  (always)
  [2] rust      https://.../rust/CLAUDE.md         (cached, 23 min old)  (found Cargo.toml at /Users/grayton/projects/my-service)

Hook status: installed (SessionStart in ~/.claude/settings.json)
```

### List all registered URLs

```sh
brief list
```

```
 Language  URL                                                   Cached   Last Fetch
 ────────  ────────────────────────────────────────────────────  ───────  ──────────────────
 baseline  https://.../CLAUDE.md                                 ✓        2026-03-10 09:14
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

Publish a `.brief.toml` file to a URL your team can reach. The format mirrors `~/.brief/config.toml`. See the [Composable agent standards](#composable-agent-standards) section above for the recommended file layout and naming conventions.

A minimal example:

```toml
[global]
cache_ttl = 3600

[languages.baseline]
url    = "https://raw.githubusercontent.com/ORG/REPO/main/standards/AGENT.md"
detect = []

[languages.rust]
url    = "https://raw.githubusercontent.com/ORG/REPO/main/standards/rust/AGENT.md"
detect = ["Cargo.toml"]

[languages.python]
url    = "https://raw.githubusercontent.com/ORG/REPO/main/standards/python/AGENT.md"
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
cache_ttl       = 3600           # Cache TTL in seconds (default: 3600)
team_config_url = "https://..."  # URL for 'brief sync' with no arguments

[languages.baseline]
url    = "https://..."
detect = []                      # Empty detect = always injected

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
