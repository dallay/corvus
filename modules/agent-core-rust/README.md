<p align="center">
  <img src="corvus.png" alt="Corvus" width="200" />
</p>

<h1 align="center">Corvus 🦀</h1>

<p align="center">
  <strong>Zero overhead. Zero compromise. 100% Rust. 100% Agnostic.</strong><br>
  ⚡️ <strong>Runs on $10 hardware with <5MB RAM: That's 99% less memory than OpenClaw and 98% cheaper than a Mac mini!</strong>
</p>

<p align="center">
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue.svg" alt="License: MIT" /></a>
  <a href="https://buymeacoffee.com/argenistherose"><img src="https://img.shields.io/badge/Buy%20Me%20a%20Coffee-Donate-yellow.svg?style=flat&logo=buy-me-a-coffee" alt="Buy Me a Coffee" /></a>
</p>

Fast, small, and fully autonomous AI assistant infrastructure — deploy anywhere, swap anything.

```
~3.4MB binary · <10ms startup · 1,017 tests · 22+ providers · 8 traits · Pluggable everything
```

### ✨ Features

- 🏎️ **Ultra-Lightweight:** <5MB Memory footprint — 99% smaller than OpenClaw core.
- 💰 **Minimal Cost:** Efficient enough to run on $10 Hardware — 98% cheaper than a Mac mini.
- ⚡ **Lightning Fast:** 400X Faster startup time, boot in <10ms (under 1s even on 0.6GHz cores).
- 🌍 **True Portability:** Single self-contained binary across ARM, x86, and RISC-V.

### Why teams pick Corvus

- **Lean by default:** small Rust binary, fast startup, low memory footprint.
- **Secure by design:** pairing, strict sandboxing, explicit allowlists, workspace scoping.
- **Fully swappable:** core systems are traits (providers, channels, tools, memory, tunnels).
- **No lock-in:** OpenAI-compatible provider support + pluggable custom endpoints.

## Benchmark Snapshot (Corvus vs OpenClaw)

Local machine quick benchmark (macOS arm64, Feb 2026) normalized for 0.8GHz edge hardware.

| | OpenClaw | NanoBot | PicoClaw | Corvus 🦀 |
|---|---|---|---|---|
| **Language** | TypeScript | Python | Go | **Rust** |
| **RAM** | > 1GB | > 100MB | < 10MB | **< 5MB** |
| **Startup (0.8GHz core)** | > 500s | > 30s | < 1s | **< 10ms** |
| **Binary Size** | ~28MB (dist) | N/A (Scripts) | ~8MB | **3.4 MB** |
| **Cost** | Mac Mini $599 | Linux SBC ~$50 | Linux Board $10 | **Any hardware $10** |

> Notes: Corvus results measured with `/usr/bin/time -l` on release builds. OpenClaw requires Node.js runtime (~390MB overhead). PicoClaw and Corvus are static binaries.

<p align="center">
  <img src="zero-claw.jpeg" alt="Corvus vs OpenClaw Comparison" width="800" />
</p>

Reproduce Corvus numbers locally:

```bash
cargo build --release
ls -lh target/release/corvus

/usr/bin/time -l target/release/corvus --help
/usr/bin/time -l target/release/corvus status
```

## Quick Start

```bash
git clone https://github.com/dallay/corvus.git
cd corvus
cargo build --release
cargo install --path . --force

# Quick setup (no prompts)
corvus onboard --api-key sk-... --provider openrouter

# Or interactive wizard
corvus onboard --interactive

# Or quickly repair channels/allowlists only
corvus onboard --channels-only

# Chat
corvus agent -m "Hello, Corvus!"

# Interactive mode
corvus agent

# Start the gateway (webhook server)
corvus gateway                # default: 127.0.0.1:8080
corvus gateway --port 0       # random port (security hardened)

# Start full autonomous runtime
corvus daemon

# Check status
corvus status

# Run system diagnostics
corvus doctor

# Check channel health
corvus channel doctor

# Get integration setup details
corvus integrations info Telegram

# Manage background service
corvus service install
corvus service status

# Migrate memory from OpenClaw (safe preview first)
corvus migrate openclaw --dry-run
corvus migrate openclaw
```

> **Dev fallback (no global install):** prefix commands with `cargo run --release --` (example: `cargo run --release -- status`).

## Architecture

Every subsystem is a **trait** — swap implementations with a config change, zero code changes.

<p align="center">
  <img src="docs/architecture.svg" alt="Corvus Architecture" width="900" />
</p>

| Subsystem | Trait | Ships with | Extend |
|-----------|-------|------------|--------|
| **AI Models** | `Provider` | 22+ providers (OpenRouter, Anthropic, OpenAI, Ollama, Venice, Groq, Mistral, xAI, DeepSeek, Together, Fireworks, Perplexity, Cohere, Bedrock, etc.) | `custom:https://your-api.com` — any OpenAI-compatible API |
| **Channels** | `Channel` | CLI, Telegram, Discord, Slack, iMessage, Matrix, WhatsApp, Webhook | Any messaging API |
| **Memory** | `Memory` | SQLite with hybrid search (FTS5 + vector cosine similarity), Markdown | Any persistence backend |
| **Tools** | `Tool` | shell, file_read, file_write, memory_store, memory_recall, memory_forget, browser_open (Brave + allowlist), composio (optional) | Any capability |
| **Observability** | `Observer` | Noop, Log, Multi | Prometheus, OTel |
| **Runtime** | `RuntimeAdapter` | Native, Docker (sandboxed) | WASM (planned; unsupported kinds fail fast) |
| **Security** | `SecurityPolicy` | Gateway pairing, sandbox, allowlists, rate limits, filesystem scoping, encrypted secrets | — |
| **Identity** | `IdentityConfig` | OpenClaw (markdown), AIEOS v1.1 (JSON) | Any identity format |
| **Tunnel** | `Tunnel` | None, Cloudflare, Tailscale, ngrok, Custom | Any tunnel binary |
| **Heartbeat** | Engine | HEARTBEAT.md periodic tasks | — |
| **Skills** | Loader | TOML manifests + SKILL.md instructions | Community skill packs |
| **Integrations** | Registry | 50+ integrations across 9 categories | Plugin system |

### Runtime support (current)

- ✅ Supported today: `runtime.kind = "native"` or `runtime.kind = "docker"`
- 🚧 Planned, not implemented yet: WASM / edge runtimes

When an unsupported `runtime.kind` is configured, Corvus now exits with a clear error instead of silently falling back to native.

### Memory System (Full-Stack Search Engine)

All custom, zero external dependencies — no Pinecone, no Elasticsearch, no LangChain:

| Layer | Implementation |
|-------|---------------|
| **Vector DB** | Embeddings stored as BLOB in SQLite, cosine similarity search |
| **Keyword Search** | FTS5 virtual tables with BM25 scoring |
| **Hybrid Merge** | Custom weighted merge function (`vector.rs`) |
| **Embeddings** | `EmbeddingProvider` trait — OpenAI, custom URL, or noop |
| **Chunking** | Line-based markdown chunker with heading preservation |
| **Caching** | SQLite `embedding_cache` table with LRU eviction |
| **Safe Reindex** | Rebuild FTS5 + re-embed missing vectors atomically |

The agent automatically recalls, saves, and manages memory via tools.

```toml
[memory]
backend = "sqlite"          # "sqlite", "markdown", "none"
auto_save = true
embedding_provider = "openai"
vector_weight = 0.7
keyword_weight = 0.3
```

## Security

Corvus enforces security at **every layer** — not just the sandbox. It passes all items from the community security checklist.

### Security Checklist

| # | Item | Status | How |
|---|------|--------|-----|
| 1 | **Gateway not publicly exposed** | ✅ | Binds `127.0.0.1` by default. Refuses `0.0.0.0` without tunnel or explicit `allow_public_bind = true`. |
| 2 | **Pairing required** | ✅ | 6-digit one-time code on startup. Exchange via `POST /pair` for bearer token. All `/webhook` requests require `Authorization: Bearer <token>`. |
| 3 | **Filesystem scoped (no /)** | ✅ | `workspace_only = true` by default. 14 system dirs + 4 sensitive dotfiles blocked. Null byte injection blocked. Symlink escape detection via canonicalization + resolved-path workspace checks in file read/write tools. |
| 4 | **Access via tunnel only** | ✅ | Gateway refuses public bind without active tunnel. Supports Tailscale, Cloudflare, ngrok, or any custom tunnel. |

> **Run your own nmap:** `nmap -p 1-65535 <your-host>` — Corvus binds to localhost only, so nothing is exposed unless you explicitly configure a tunnel.

### Channel allowlists (Telegram / Discord / Slack)

Inbound sender policy is now consistent:

- Empty allowlist = **deny all inbound messages**
- `"*"` = **allow all** (explicit opt-in)
- Otherwise = exact-match allowlist

This keeps accidental exposure low by default.

Recommended low-friction setup (secure + fast):

- **Telegram:** allowlist your own `@username` (without `@`) and/or your numeric Telegram user ID.
- **Discord:** allowlist your own Discord user ID.
- **Slack:** allowlist your own Slack member ID (usually starts with `U`).
- Use `"*"` only for temporary open testing.

If you're not sure which identity to use:

1. Start channels and send one message to your bot.
2. Read the warning log to see the exact sender identity.
3. Add that value to the allowlist and rerun channels-only setup.

If you hit authorization warnings in logs (for example: `ignoring message from unauthorized user`),
rerun channel setup only:

```bash
corvus onboard --channels-only
```

### WhatsApp Business Cloud API Setup

WhatsApp uses Meta's Cloud API with webhooks (push-based, not polling):

1. **Create a Meta Business App:**
   - Open the WhatsApp Business Platform page from Meta and create an app
   - Create a new app → Select "Business" type
   - Add the "WhatsApp" product

2. **Get your credentials:**
   - **Access Token:** From WhatsApp → API Setup → Generate token (or create a System User for permanent tokens)
   - **Phone Number ID:** From WhatsApp → API Setup → Phone number ID
   - **Verify Token:** You define this (any random string) — Meta will send it back during webhook verification

3. **Configure Corvus:**
   ```toml
   [channels_config.whatsapp]
   access_token = "EAABx..."
   phone_number_id = "123456789012345"
   verify_token = "my-secret-verify-token"
   allowed_numbers = ["+1234567890"]  # E.164 format, or ["*"] for all
   ```

4. **Start the gateway with a tunnel:**
   ```bash
   corvus gateway --port 8080
   ```
   WhatsApp requires HTTPS, so use a tunnel (ngrok, Cloudflare, Tailscale Funnel).

5. **Configure Meta webhook:**
   - In Meta Developer Console → WhatsApp → Configuration → Webhook
   - **Callback URL:** `https://your-tunnel-url/whatsapp`
   - **Verify Token:** Same as your `verify_token` in config
   - Subscribe to `messages` field

6. **Test:** Send a message to your WhatsApp Business number — Corvus will respond via the LLM.

## Configuration

Config: `~/.corvus/config.toml` (created by `onboard`)

```toml
api_key = "sk-..."
default_provider = "openrouter"
default_model = "anthropic/claude-sonnet-4-20250514"
default_temperature = 0.7

[memory]
backend = "sqlite"              # "sqlite", "markdown", "none"
auto_save = true
embedding_provider = "openai"   # "openai", "noop"
vector_weight = 0.7
keyword_weight = 0.3

[gateway]
require_pairing = true          # require pairing code on first connect
allow_public_bind = false       # refuse 0.0.0.0 without tunnel

[autonomy]
level = "supervised"            # "readonly", "supervised", "full" (default: supervised)
workspace_only = true           # default: true — scoped to workspace
allowed_commands = ["git", "npm", "cargo", "ls", "cat", "grep"]
forbidden_paths = ["/etc", "/root", "/proc", "/sys", "~/.ssh", "~/.gnupg", "~/.aws"]

[runtime]
kind = "native"                # "native" or "docker"

[runtime.docker]
image = "alpine:3.20"          # container image for shell execution
network = "none"               # docker network mode ("none", "bridge", etc.)
memory_limit_mb = 512          # optional memory limit in MB
cpu_limit = 1.0                # optional CPU limit
read_only_rootfs = true        # mount root filesystem as read-only
mount_workspace = true         # mount workspace into /workspace
allowed_workspace_roots = []   # optional allowlist for workspace mount validation

[heartbeat]
enabled = false
interval_minutes = 30

[tunnel]
provider = "none"               # "none", "cloudflare", "tailscale", "ngrok", "custom"

[secrets]
encrypt = true                  # API keys encrypted with local key file

[browser]
enabled = false                 # opt-in browser_open tool
allowed_domains = ["docs.rs"]  # required when browser is enabled

[composio]
enabled = false                 # opt-in: 1000+ OAuth apps via composio.dev

[identity]
format = "openclaw"             # "openclaw" (default, markdown files) or "aieos" (JSON)
# aieos_path = "identity.json"  # path to AIEOS JSON file (relative to workspace or absolute)
# aieos_inline = '{"identity":{"names":{"first":"Nova"}}}'  # inline AIEOS JSON
```

## Identity System (AIEOS Support)

Corvus supports **identity-agnostic** AI personas through two formats:

### OpenClaw (Default)

Traditional markdown files in your workspace:
- `IDENTITY.md` — Who the agent is
- `SOUL.md` — Core personality and values
- `USER.md` — Who the agent is helping
- `AGENTS.md` — Behavior guidelines

### AIEOS (AI Entity Object Specification)

[AIEOS](https://aieos.org) is a standardization framework for portable AI identity. Corvus supports AIEOS v1.1 JSON payloads, allowing you to:

- **Import identities** from the AIEOS ecosystem
- **Export identities** to other AIEOS-compatible systems
- **Maintain behavioral integrity** across different AI models

#### Enable AIEOS

```toml
[identity]
format = "aieos"
aieos_path = "identity.json"  # relative to workspace or absolute path
```

Or inline JSON:

```toml
[identity]
format = "aieos"
aieos_inline = '''
{
  "identity": {
    "names": { "first": "Nova", "nickname": "N" }
  },
  "psychology": {
    "neural_matrix": { "creativity": 0.9, "logic": 0.8 },
    "traits": { "mbti": "ENTP" },
    "moral_compass": { "alignment": "Chaotic Good" }
  },
  "linguistics": {
    "text_style": { "formality_level": 0.2, "slang_usage": true }
  },
  "motivations": {
    "core_drive": "Push boundaries and explore possibilities"
  }
}
'''
```

#### AIEOS Schema Sections

| Section | Description |
|---------|-------------|
| `identity` | Names, bio, origin, residence |
| `psychology` | Neural matrix (cognitive weights), MBTI, OCEAN, moral compass |
| `linguistics` | Text style, formality, catchphrases, forbidden words |
| `motivations` | Core drive, short/long-term goals, fears |
| `capabilities` | Skills and tools the agent can access |
| `physicality` | Visual descriptors for image generation |
| `history` | Origin story, education, occupation |
| `interests` | Hobbies, favorites, lifestyle |

See [aieos.org](https://aieos.org) for the full schema and live examples.

## Gateway API

| Endpoint | Method | Auth | Description |
|----------|--------|------|-------------|
| `/health` | GET | None | Health check (always public, no secrets leaked) |
| `/pair` | POST | `X-Pairing-Code` header | Exchange one-time code for bearer token |
| `/webhook` | POST | `Authorization: Bearer <token>` | Send message: `{"message": "your prompt"}` |
| `/whatsapp` | GET | Query params | Meta webhook verification (hub.mode, hub.verify_token, hub.challenge) |
| `/whatsapp` | POST | None (Meta signature) | WhatsApp incoming message webhook |

## Commands

| Command | Description |
|---------|-------------|
| `onboard` | Quick setup (default) |
| `onboard --interactive` | Full interactive 7-step wizard |
| `onboard --channels-only` | Reconfigure channels/allowlists only (fast repair flow) |
| `agent -m "..."` | Single message mode |
| `agent` | Interactive chat mode |
| `gateway` | Start webhook server (default: `127.0.0.1:8080`) |
| `gateway --port 0` | Random port mode |
| `daemon` | Start long-running autonomous runtime |
| `service install/start/stop/status/uninstall` | Manage user-level background service |
| `doctor` | Diagnose daemon/scheduler/channel freshness |
| `status` | Show full system status |
| `channel doctor` | Run health checks for configured channels |
| `integrations info <name>` | Show setup/status details for one integration |

## Development

```bash
cargo build              # Dev build
cargo build --release    # Release build (~3.4MB)
cargo test               # 1,017 tests
cargo clippy             # Lint (0 warnings)
cargo fmt                # Format

# Run the SQLite vs Markdown benchmark
cargo test --test memory_comparison -- --nocapture
```

### Pre-push hook

A git hook runs `cargo fmt --check`, `cargo clippy -- -D warnings`, and `cargo test` before every push. Enable it once:

```bash
git config core.hooksPath .githooks
```

To skip the hook when you need a quick push during development:

```bash
git push --no-verify
```

## Support

Corvus is an open-source project maintained with passion. If you find it useful and would like to support its continued development, hardware for testing, and coffee for the maintainer, you can support me here:

<a href="https://buymeacoffee.com/argenistherose"><img src="https://img.shields.io/badge/Buy%20Me%20a%20Coffee-Donate-yellow.svg?style=for-the-badge&logo=buy-me-a-coffee" alt="Buy Me a Coffee" /></a>

## License

MIT — see [LICENSE](LICENSE)

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Implement a trait, submit a PR:
- CI workflow guide: [docs/ci-map.md](docs/ci-map.md)
- New `Provider` → `src/providers/`
- New `Channel` → `src/channels/`
- New `Observer` → `src/observability/`
- New `Tool` → `src/tools/`
- New `Memory` → `src/memory/`
- New `Tunnel` → `src/tunnel/`
- New `Skill` → `~/.corvus/workspace/skills/<name>/`

---

**Corvus** — Zero overhead. Zero compromise. Deploy anywhere. Swap anything. 🦀
