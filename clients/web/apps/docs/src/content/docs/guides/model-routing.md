---
title: Model Routing & Query Classification
description: Configure multi-model routing with task hints and automatic query classification in the Corvus agent runtime.
owner: team-platform
status: canonical
lastReviewed: 2026-04-07
appliesTo: main
docType: guide
---

Corvus can route different requests to different providers and models without changing your
application code. You define named route hints in TOML, optionally classify messages into those
hints, and verify the setup with `corvus doctor`.

Use this guide when you want to:

- send fast prompts to a cheaper model,
- reserve deeper reasoning for a stronger model,
- send code questions to a code-specialized model,
- route image turns only to routes that explicitly allow image input.

## What model routing does

`[[model_routes]]` maps a hint name to a provider and model.

```toml
[[model_routes]]
hint = "fast"
provider = "groq"
model = "llama-3.3-70b-versatile"

[[model_routes]]
hint = "reasoning"
provider = "openai"
model = "o1-preview"
```

At request time Corvus can receive a selector like `hint:reasoning`. That selector resolves to the
matching route and dispatches the request to the configured provider and model.

If no route matches, Corvus falls back to the default provider. The runtime keeps working, but it
logs a warning so you can fix the configuration.

## What query classification does

`[query_classification]` is optional. When enabled, it examines the user message and tries to pick
the best route hint automatically.

```toml
[query_classification]
enabled = true

[[query_classification.rules]]
hint = "code"
keywords = ["bug", "stack trace", "debug"]
patterns = ["fn ", "```", "Exception"]
priority = 20
```

If no rule matches, Corvus uses the default model. That is normal behavior and does not require a
warning.

## Hint flow end to end

```text
User message
  ↓
Query classification enabled?
  ├─ No  → use default model
  └─ Yes
       ↓
  Rules evaluated by priority
       ↓
  Matching rule found?
  ├─ No  → use default model
  └─ Yes → emit hint string (example: "reasoning")
               ↓
          Router resolves hint against [[model_routes]]
               ↓
          Route found?
          ├─ Yes → dispatch to that provider + model
          └─ No  → log warning and fall back to default provider
```

## Config reference

### `[[model_routes]]`

Each entry defines one named route.

| Field | Type | Required | Default | Purpose | Example |
|---|---|---:|---|---|---|
| `hint` | string | Yes | none | Name used by classification or direct `hint:<name>` selection. | `"reasoning"` |
| `provider` | string | Yes | none | Provider Corvus should dispatch to for this route. | `"openai"` |
| `model` | string | Yes | none | Model name passed to that provider. | `"o1-preview"` |
| `api_key` | string | No | unset | Optional per-route credential override for this provider. | `"env:OPENAI_KEY"` or a literal secret if you manage secrets that way |
| `allow_image_input` | boolean | No | `false` | Opt-in gate for image turns on this route. | `true` |

#### Notes

- `hint` values should be short, stable names such as `fast`, `reasoning`, `code`, or `vision`.
- `provider` must match a provider name Corvus can initialize.
- `allow_image_input` is opt-in. If you omit it, the route is treated as text-only.

### `[query_classification]`

This section controls whether Corvus tries to choose hints automatically.

| Field | Type | Required | Default | Purpose | Example |
|---|---|---:|---|---|---|
| `enabled` | boolean | No | `false` | Enables automatic hint selection from message content. | `true` |
| `rules` | array | No | `[]` | Ordered list of classification rules. | `[[query_classification.rules]]` |

### `[[query_classification.rules]]`

Each rule can match by keyword, literal pattern, or both.

| Field | Type | Required | Default | Purpose | Example |
|---|---|---:|---|---|---|
| `hint` | string | Yes | none | Route hint to return when this rule matches. Must match a `[[model_routes]]` hint. | `"code"` |
| `keywords` | array of strings | No | `[]` | Case-insensitive substring matches. | `["debug", "bug"]` |
| `patterns` | array of strings | No | `[]` | Case-sensitive literal matches. Good for code fragments. | `["fn ", "```rust"]` |
| `min_length` | integer | No | unset | Only match when the message length is at least this value. | `40` |
| `max_length` | integer | No | unset | Only match when the message length is at most this value. | `500` |
| `priority` | integer | No | `0` | Higher numbers are checked first. | `20` |

#### Rule behavior

- Length checks run first.
- After that, the rule matches if **any** keyword matches or **any** pattern matches.
- Keywords are case-insensitive.
- Patterns are case-sensitive.
- The first matching rule by descending priority wins.

## Example: fast and reasoning split

```toml
default_provider = "openai"
default_model = "gpt-4o-mini"

[[model_routes]]
hint = "fast"
provider = "groq"
model = "llama-3.3-70b-versatile"

[[model_routes]]
hint = "reasoning"
provider = "openai"
model = "o1-preview"
```

Use `hint:fast` for low-latency responses and `hint:reasoning` for harder prompts.

## Example: code-specialized routing

```toml
default_provider = "openai"
default_model = "gpt-4o-mini"

[[model_routes]]
hint = "code"
provider = "groq"
model = "qwen-qwq-32b"

[query_classification]
enabled = true

[[query_classification.rules]]
hint = "code"
keywords = ["debug", "refactor", "stack trace", "compile error"]
patterns = ["fn ", "```", "Exception"]
priority = 20
```

## Example: vision route for image input

```toml
default_provider = "openai"
default_model = "gpt-4o-mini"

[[model_routes]]
hint = "vision"
provider = "openai"
model = "gpt-4o"
allow_image_input = true

[multimodal]
enabled = true
vision_model_hint = "vision"
```

Image turns are accepted only when `vision_model_hint` resolves to a route with
`allow_image_input = true`.

## Example: multi-provider routing with classification

```toml
default_provider = "openrouter"
default_model = "openai/gpt-4o-mini"

[[model_routes]]
hint = "fast"
provider = "groq"
model = "llama-3.3-70b-versatile"

[[model_routes]]
hint = "reasoning"
provider = "openai"
model = "o1-preview"

[[model_routes]]
hint = "code"
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[query_classification]
enabled = true

[[query_classification.rules]]
hint = "reasoning"
keywords = ["compare", "tradeoff", "strategy"]
priority = 30

[[query_classification.rules]]
hint = "code"
keywords = ["debug", "stack trace", "refactor"]
patterns = ["fn ", "```"]
priority = 20

[[query_classification.rules]]
hint = "fast"
keywords = ["summarize", "quick", "brief"]
priority = 10
```

## Validate the configuration

Run:

```bash
corvus doctor
```

For routing and classification, Phase 1 adds these warnings:

- classification enabled but no rules are configured,
- classification enabled but no `[[model_routes]]` exist,
- a classification rule points to a hint that is not present in `[[model_routes]]`,
- a rule has neither keywords nor patterns and can never match.

Warnings do **not** block startup. They tell you the configuration is valid enough to run, but not
fully aligned with the routing contract.

## Troubleshooting

| Symptom | Likely cause | What `corvus doctor` or logs tell you | Resolution |
|---|---|---|---|
| Classification never changes the selected model. | `enabled = true` but `rules` is empty. | Warning: classification is enabled but no rules are configured. | Add at least one rule or disable classification. |
| Classification returns hints but routing still falls back to the default provider. | A rule points to a hint that does not exist in `[[model_routes]]`. | Warning names the orphaned hint and the available route hints. | Change the rule hint to a real route, or add the missing route. |
| A rule never fires, even though classification is enabled. | The rule has empty `keywords` and empty `patterns`. | Warning names the affected hint and says the rule will never match. | Add keywords, patterns, or remove the rule. |
| Image turns are rejected. | The route used by `vision_model_hint` does not set `allow_image_input = true`. | Runtime rejects the turn for a non-image-capable route. | Point `vision_model_hint` at a route with `allow_image_input = true`. |
| A direct selector like `hint:code` does not route as expected. | The route hint is unknown. | Runtime warning says the hint is unknown and that Corvus is falling back to the default provider with the raw model string. | Fix the hint name or add the missing route. |
| A route works in config but fails at request time. | A non-primary provider failed during initialization. | Runtime warning names the failed provider and the affected route hints. | Fix that provider's credentials or configuration, then restart and re-run `corvus doctor`. |

## Operator checklist

- Define at least one `[[model_routes]]` entry for every hint you plan to use.
- Keep hint names identical between routes and classification rules.
- Use `priority` to make the most specific rule win first.
- Opt in to `allow_image_input = true` only on routes that should accept image turns.
- Run `corvus doctor` after configuration changes.

If you want to start simple, create only `fast` and `reasoning` routes first, verify them with
`corvus doctor`, and add classification rules after the base routing works.
