# Exploration: Productize Model Routing and Query Classification

### Current State

The Corvus agent-runtime has a functional model routing and query classification system spread across four key files:

#### 1. Config Schema (`config/schema.rs`)

**`ModelRouteConfig`** — defines a route mapping a hint name to a provider+model pair:
- `hint: String` — task hint name (e.g. "reasoning", "fast", "code")
- `provider: String` — provider to route to (must match a known provider name)
- `model: String` — model to use with that provider
- `api_key: Option<String>` — optional API key override per route
- `allow_image_input: bool` — explicit opt-in for multimodal image routing (default: false)

**`QueryClassificationConfig`** — automatic classification of user messages:
- `enabled: bool` — disabled by default
- `rules: Vec<ClassificationRule>` — ordered list of classification rules

**`ClassificationRule`** — maps message patterns to a hint:
- `hint: String` — must match a `[[model_routes]]` hint value
- `keywords: Vec<String>` — case-insensitive substring matches
- `patterns: Vec<String>` — case-sensitive literal matches (e.g. "fn ", "```")
- `min_length: Option<usize>` — only match if message >= N chars
- `max_length: Option<usize>` — only match if message <= N chars
- `priority: i32` — higher priority rules are checked first (default: 0)

#### 2. Classifier (`agent/classifier.rs`)

Pure function `classify(config, message) -> Option<String>`:
- Returns `None` when disabled, no rules configured, or no match
- Sorts rules by priority (descending) before checking
- Checks length constraints first, then keyword/pattern match
- Keywords are case-insensitive (pre-normalized at config load time)
- Patterns are case-sensitive (designed for code patterns like "fn ", "```")

#### 3. Router (`providers/router.rs`)

`RouterProvider` wraps multiple providers and routes by hint:
- Model parameter `"hint:reasoning"` → strips prefix, looks up route table → dispatches to mapped provider+model
- Unknown hint → falls back to default provider with the raw model string (including `hint:` prefix)
- Non-hint model strings (e.g. `"gpt-4o"`) → default provider with that model
- Merges capabilities across all wrapped providers
- Fail-closed for image routing: rejects image turns to non-image-capable providers
- Warmup calls all wrapped providers

#### 4. Agent Integration (`agent/agent.rs`)

- `classify_model(user_message)` called during `prepare_turn_with_context`
- If classifier returns a hint AND the hint exists in `available_hints` (from `model_routes`), returns `"hint:{hint}"`
- Otherwise returns the default `model_name`
- Cost tracking resolves `hint:` prefix for pricing lookups
- Available hints collected from `config.model_routes` at agent construction

#### 5. Provider Factory (`providers/mod.rs`)

`create_routed_provider()`:
- If no model_routes configured → returns standard resilient provider
- Collects unique provider names from routes + primary
- Creates each provider with its own resilience wrapper (retry/fallback)
- Route-specific `api_key` override supported (trimmed, non-empty)
- `api_url` override only applied to the primary provider
- Non-primary providers that fail to initialize are silently skipped (warning logged)

#### 6. Channel Image Routing (`channels/mod.rs`)

- Vision model hint from `multimodal.vision_model_hint` resolves to a matching `model_routes` entry
- Cross-validates `allow_image_input=true` on the matched route
- Returns `ResolvedImageRoute` with `selector: "hint:{hint}"`, provider, and model

#### 7. Doctor Checks (`doctor/mod.rs`)

`check_model_routes()` validates:
- Route hint is not empty
- Route provider is a valid provider name
- Route model is not empty

### Affected Areas

- `clients/agent-runtime/src/config/schema.rs` — config types and validation
- `clients/agent-runtime/src/agent/classifier.rs` — classification logic
- `clients/agent-runtime/src/providers/router.rs` — route resolution
- `clients/agent-runtime/src/providers/mod.rs` — provider factory/wiring
- `clients/agent-runtime/src/agent/agent.rs` — agent integration
- `clients/agent-runtime/src/channels/mod.rs` — image routing
- `clients/agent-runtime/src/doctor/mod.rs` — diagnostics
- `clients/agent-runtime/src/onboard/wizard.rs` — onboarding (currently no routing setup)
- `clients/web/apps/docs/` — documentation site

### Current Documentation Coverage

**Virtually zero dedicated documentation:**
- `architecture.md` has ONE sentence: "The routing system (`providers/router.rs`) can direct requests to different providers based on configuration, cost, or availability."
- No dedicated routing/classification guide in the docs site
- No config reference for `[[model_routes]]` or `[query_classification]`
- No example TOML snippets in docs (only in code comments on the struct)
- No CLI reference for routing-related diagnostics
- No spec in `openspec/specs/` related to routing

### Current Test Coverage

**Well-tested at unit level:**

| Module | Tests | Coverage Focus |
|--------|-------|----------------|
| `classifier.rs` | 6 tests | disabled, empty rules, keyword case-insensitive, pattern case-sensitive, length constraints, priority ordering, no-match |
| `router.rs` | 12 tests | hint routing, fast hint, unknown hint fallback, non-hint passthrough, resolve behavior, unknown provider skip, warmup, system prompt passthrough, tool routing, image rejection, text-only passthrough, capability merging |
| `schema.rs` | 6+ tests | route api_key override, classification default disabled, rule length constraints, rule defaults, TOML parsing, image input default/opt-in |
| `doctor/mod.rs` | 1+ test | route validation diagnostics |
| `channels/mod.rs` | 4+ tests | vision route resolution, image rejection paths |

**Missing test coverage:**
- No integration test for end-to-end classify → route → provider dispatch
- No test for classification hint that doesn't exist in available_hints (the safety check in `classify_model`)
- No test for `normalize_query_classification_keywords` during config load
- No test for route with api_key override actually being used during provider creation

### Product Gaps

#### Gap 1: No Operator Documentation
Operators cannot discover how to configure routing without reading Rust source code. There's no guide explaining:
- What `[[model_routes]]` is and how to configure it
- What `[query_classification]` is and how to set up rules
- Example TOML configurations for common scenarios (fast/reasoning split, code model, vision)
- How hints flow through the system (classification → hint → router → provider)
- How to verify routing works (`corvus doctor` output, logs)

#### Gap 2: No Config Validation for Classification ↔ Route Integrity
- A classification rule can reference a hint that doesn't exist in `model_routes` — silently falls back to default model at runtime
- No doctor check validates that classification rule hints match defined routes
- No doctor check validates that `query_classification.enabled=true` with no rules is pointless
- No warning when classification is enabled but no routes are configured

#### Gap 3: No Dry-Run / Diagnostic CLI
- No `corvus route test "message"` to see which hint a message would classify to
- No `corvus route list` to see configured routes and their status
- No `corvus route check` to validate classification ↔ route integrity
- Doctor output for routes is minimal (just checks provider validity and empty hint/model)

#### Gap 4: No Observability for Routing Decisions
- Classification result is logged (`tracing::info!`) but not emitted as a structured observer event
- No metrics for route usage distribution (which hints are being triggered)
- No metrics for classification miss rate (how often no rule matches)
- No way for operators to audit routing behavior after the fact

#### Gap 5: Silent Failure Modes
- Unknown hint falls back to default with the raw `"hint:nonexistent"` as the model name — this will likely cause a provider error with a confusing message
- Non-primary providers that fail to initialize are silently skipped — routes referencing them will fail at request time
- Classification rule with empty keywords AND empty patterns will never match — no warning

#### Gap 6: Onboarding Doesn't Surface Routing
- The setup wizard (`onboard/wizard.rs`) creates configs with empty `model_routes` and default `query_classification`
- No wizard step for configuring routing even for multi-provider setups
- Operators who set up both Ollama and OpenRouter don't get prompted about routing

#### Gap 7: No Spec Exists
- No formal spec in `openspec/specs/` for routing behavior
- This means no contractual definition of what the routing system promises

### Approaches

1. **Documentation-First Productization** — Write comprehensive docs, add config validation, add doctor checks
   - Pros: Low risk, immediately useful, no runtime changes
   - Cons: Doesn't address diagnostic CLI or observability gaps
   - Effort: Medium

2. **Full Productization** — Docs + validation + diagnostic CLI + observability + onboarding
   - Pros: Complete operator experience, fully discoverable
   - Cons: Larger scope, more changes across modules
   - Effort: High

3. **Incremental Productization** — Phase 1: docs + validation + doctor. Phase 2: CLI diagnostics + observability. Phase 3: onboarding integration
   - Pros: Delivers value incrementally, lower risk per phase
   - Cons: Takes longer for full completion
   - Effort: Medium per phase

### Recommendation

**Approach 3: Incremental Productization** — This is a planning issue, and the existing runtime code is solid. The gap is entirely in operator experience. Breaking into phases:

- **Phase 1** (this change): Operator documentation guide + config validation + enhanced doctor checks + formal spec
- **Phase 2** (follow-up): Diagnostic CLI commands + structured observability events
- **Phase 3** (follow-up): Onboarding wizard integration + example config templates

Phase 1 gives operators everything they need to discover, configure, and validate routing today. Phases 2-3 can be separate issues.

### Risks

- **Low**: Documentation changes are zero-risk to runtime behavior
- **Low**: Config validation additions are additive (new warnings, not breaking changes)
- **Medium**: If we add fail-hard validation (e.g. classification hint must match route), existing configs with orphaned hints would break — should be warnings first, errors in a future major version
- **Low**: Doctor check additions are purely additive

### Ready for Proposal

Yes — the codebase is well-understood, the gaps are clear, and the incremental approach gives a concrete Phase 1 scope. The proposal should focus on Phase 1 deliverables: documentation, config validation, doctor checks, and a formal routing spec.
