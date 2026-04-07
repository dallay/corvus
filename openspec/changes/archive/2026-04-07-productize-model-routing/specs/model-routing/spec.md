# Delta for Model Routing — Phase 1 Productization

**Change**: productize-model-routing
**Issue**: DALLAY-173 (GitHub #269)
**Date**: 2026-04-07
**Cross-references**: `provider-vision-gating` spec (REQ-6 for image routing gating)

---

## ADDED Requirements

### Requirement: Operator Documentation Guide

The system MUST provide a dedicated operator documentation page in the docs site
(`clients/web/apps/docs/`) that enables operators to configure, validate, and troubleshoot
model routing and query classification without reading source code.

The documentation guide MUST cover:

1. **Config reference** — complete TOML reference for `[[model_routes]]` fields (`hint`,
   `provider`, `model`, `api_key`, `allow_image_input`) and `[query_classification]` fields
   (`enabled`, `rules` with `hint`, `keywords`, `patterns`, `min_length`, `max_length`,
   `priority`).
2. **Example configurations** — at minimum: fast/reasoning split, code-specialized model,
   vision routing with `allow_image_input`, and a multi-provider setup with classification.
3. **Hint flow explanation** — how a user message flows through classification → hint
   resolution → router dispatch → provider, including what happens when classification is
   disabled or no rule matches.
4. **Troubleshooting section** — common misconfigurations and their symptoms, including
   orphaned classification hints, empty rule sets, never-matching rules, and unknown hint
   fallback behavior.

The documentation SHOULD include a visual or textual diagram of the hint flow.

The documentation MUST NOT expose internal implementation details (struct names, module paths)
beyond what is necessary for operator understanding.

#### Scenario: Operator configures routing using only documentation

- GIVEN an operator with no prior knowledge of Corvus routing internals
- WHEN they read the model routing documentation guide
- THEN they MUST be able to write a valid `[[model_routes]]` configuration with at least two
  routes (e.g., fast and reasoning)
- AND they MUST be able to write a valid `[query_classification]` configuration with at least
  one rule
- AND they MUST be able to verify their configuration using `corvus doctor`

#### Scenario: Documentation covers all config fields

- GIVEN the model routing documentation guide
- WHEN an operator looks up any `[[model_routes]]` or `[query_classification]` config field
- THEN the guide MUST describe the field's purpose, type, default value, and at least one
  example usage
- AND optional fields MUST be clearly marked as optional

#### Scenario: Documentation explains hint flow end-to-end

- GIVEN the model routing documentation guide
- WHEN an operator reads the hint flow section
- THEN they MUST understand the path: user message → classifier → hint string → router
  lookup → provider dispatch
- AND they MUST understand what happens when classification is disabled (default model used)
- AND they MUST understand what happens when no classification rule matches (default model
  used)

#### Scenario: Troubleshooting section covers common misconfigurations

- GIVEN the model routing documentation guide
- WHEN an operator encounters a misconfiguration (orphaned hint, empty rules, never-matching
  rule)
- THEN the troubleshooting section MUST describe the symptom, likely cause, and resolution
- AND the troubleshooting section MUST reference the relevant `corvus doctor` warning

---

### Requirement: Config Validation — Classification Rule Hint Integrity

The `corvus doctor` command MUST validate that every classification rule's `hint` field
references a hint that exists in the configured `[[model_routes]]`. If a classification rule
references a non-existent route hint, the doctor MUST emit a warning.

The validation MUST be a warning, not an error. The system MUST NOT refuse to start based on
this check.

#### Scenario: Classification rule references non-existent route hint

- GIVEN a configuration with `[[model_routes]]` defining hints `["fast", "reasoning"]`
- AND `[query_classification]` has a rule with `hint = "code"`
- WHEN the operator runs `corvus doctor`
- THEN the doctor MUST emit a warning indicating that classification rule hint `"code"` does
  not match any configured model route
- AND the warning MUST name both the orphaned hint and the available route hints

#### Scenario: All classification rule hints match configured routes

- GIVEN a configuration with `[[model_routes]]` defining hints `["fast", "reasoning"]`
- AND `[query_classification]` has rules with hints `["fast", "reasoning"]`
- WHEN the operator runs `corvus doctor`
- THEN no warning MUST be emitted for classification rule hint integrity

#### Scenario: Classification disabled — hint integrity check skipped

- GIVEN a configuration with `query_classification.enabled = false`
- AND classification rules reference non-existent route hints
- WHEN the operator runs `corvus doctor`
- THEN the doctor SHOULD NOT emit warnings for classification rule hint integrity
- AND the doctor MAY note that classification is disabled

---

### Requirement: Config Validation — Classification Enabled with Zero Rules

The `corvus doctor` command MUST emit a warning when `query_classification.enabled = true` but
the `rules` list is empty. This configuration is valid but pointless — classification will
never produce a hint.

#### Scenario: Classification enabled with zero rules

- GIVEN a configuration with `query_classification.enabled = true`
- AND `query_classification.rules` is an empty list
- WHEN the operator runs `corvus doctor`
- THEN the doctor MUST emit a warning indicating that classification is enabled but no rules
  are configured
- AND the warning SHOULD suggest adding rules or disabling classification

#### Scenario: Classification enabled with at least one rule

- GIVEN a configuration with `query_classification.enabled = true`
- AND `query_classification.rules` contains at least one rule
- WHEN the operator runs `corvus doctor`
- THEN no warning MUST be emitted for the zero-rules condition

---

### Requirement: Config Validation — Classification Enabled with Zero Model Routes

The `corvus doctor` command MUST emit a warning when `query_classification.enabled = true` but
no `[[model_routes]]` are configured. Without routes, classification hints have nowhere to
resolve.

#### Scenario: Classification enabled with zero model routes

- GIVEN a configuration with `query_classification.enabled = true`
- AND no `[[model_routes]]` entries are configured
- WHEN the operator runs `corvus doctor`
- THEN the doctor MUST emit a warning indicating that classification is enabled but no model
  routes are configured
- AND the warning SHOULD explain that classification hints require model routes to function

#### Scenario: Classification enabled with model routes present

- GIVEN a configuration with `query_classification.enabled = true`
- AND at least one `[[model_routes]]` entry is configured
- WHEN the operator runs `corvus doctor`
- THEN no warning MUST be emitted for the zero-routes condition

---

### Requirement: Config Validation — Never-Matching Classification Rule

The `corvus doctor` command MUST emit a warning when a classification rule has both an empty
`keywords` list AND an empty `patterns` list. Such a rule can never match any user message
regardless of length constraints.

#### Scenario: Classification rule with empty keywords and empty patterns

- GIVEN a configuration with a classification rule where `keywords = []` and `patterns = []`
- WHEN the operator runs `corvus doctor`
- THEN the doctor MUST emit a warning indicating that the rule for hint `"{hint}"` has no
  keywords and no patterns and will never match
- AND the warning MUST name the affected hint

#### Scenario: Classification rule with keywords but no patterns

- GIVEN a configuration with a classification rule where `keywords = ["debug"]` and
  `patterns = []`
- WHEN the operator runs `corvus doctor`
- THEN no warning MUST be emitted for the never-matching condition

#### Scenario: Classification rule with patterns but no keywords

- GIVEN a configuration with a classification rule where `keywords = []` and
  `patterns = ["fn "]`
- WHEN the operator runs `corvus doctor`
- THEN no warning MUST be emitted for the never-matching condition

---

### Requirement: Silent Failure Fix — Unknown Hint Fallback Logging

When the router receives a model selector with the `hint:` prefix and the hint does not match
any configured route, the router MUST log a warning before falling back to the default
provider.

The warning MUST include the unknown hint name and MUST describe the fallback behavior (using
default provider with the raw model string).

The router MUST NOT change its fallback behavior — it MUST continue to fall back to the
default provider. This requirement adds observability only.

#### Scenario: Unknown hint triggers warning log

- GIVEN a configuration with `[[model_routes]]` defining hints `["fast", "reasoning"]`
- WHEN the router receives a model selector `"hint:code"`
- THEN the router MUST log a warning at `WARN` level
- AND the warning MUST include the text `"code"` (the unknown hint name)
- AND the warning MUST describe that the system is falling back to the default provider
- AND the router MUST proceed to dispatch to the default provider

#### Scenario: Known hint does not trigger warning

- GIVEN a configuration with `[[model_routes]]` defining hints `["fast", "reasoning"]`
- WHEN the router receives a model selector `"hint:fast"`
- THEN the router MUST NOT log any warning for hint resolution
- AND the router MUST dispatch to the provider mapped to the `"fast"` route

#### Scenario: Non-hint model selector does not trigger warning

- GIVEN any configuration
- WHEN the router receives a model selector `"gpt-4o"` (no `hint:` prefix)
- THEN the router MUST NOT log any hint-related warning
- AND the router MUST dispatch to the default provider with `"gpt-4o"` as the model

---

### Requirement: Silent Failure Fix — Failed Provider Init Route Impact Logging

When a non-primary provider fails to initialize during `create_routed_provider()`, the system
MUST log a warning that names the affected model routes — i.e., the routes whose `provider`
field references the failed provider.

The system MUST NOT change its current behavior of skipping failed non-primary providers. This
requirement adds observability only.

#### Scenario: Failed provider init logs affected routes

- GIVEN a configuration with model routes:
  - `hint = "fast"`, `provider = "ollama"`
  - `hint = "reasoning"`, `provider = "ollama"`
  - `hint = "code"`, `provider = "openai"`
- AND the `"ollama"` provider fails to initialize
- WHEN `create_routed_provider()` runs
- THEN the system MUST log a warning at `WARN` level
- AND the warning MUST name the provider `"ollama"`
- AND the warning MUST list the affected routes: `"fast"` and `"reasoning"`
- AND the system MUST continue initialization without the `"ollama"` provider

#### Scenario: All providers initialize successfully

- GIVEN a configuration with model routes referencing providers `"openai"` and `"ollama"`
- AND both providers initialize successfully
- WHEN `create_routed_provider()` runs
- THEN no warning MUST be emitted for provider initialization failures

---

### Requirement: Formal Routing Spec — Route Resolution Contract

The formal spec at `openspec/specs/model-routing/spec.md` MUST define the route resolution
contract:

1. A model selector with the `hint:` prefix MUST be parsed by stripping the prefix and looking
   up the remaining string in the route table.
2. If the hint matches a configured route, the router MUST dispatch to the route's mapped
   provider using the route's mapped model.
3. If the hint does not match any configured route, the router MUST fall back to the default
   provider using the raw model selector string (including the `hint:` prefix).
4. A model selector without the `hint:` prefix MUST be dispatched to the default provider
   using the selector as the model name.

#### Scenario: Hint prefix routes to mapped provider and model

- GIVEN a route `hint = "reasoning"`, `provider = "openai"`, `model = "o1-preview"`
- WHEN the router receives model selector `"hint:reasoning"`
- THEN the router MUST dispatch to the `"openai"` provider with model `"o1-preview"`

#### Scenario: Unknown hint falls back to default provider

- GIVEN routes for `["fast", "reasoning"]` and default provider is `"openai"`
- WHEN the router receives model selector `"hint:unknown"`
- THEN the router MUST dispatch to the default `"openai"` provider
- AND the model string passed to the provider MUST be `"hint:unknown"`

#### Scenario: Non-hint selector uses default provider directly

- GIVEN any route configuration and default provider is `"openai"`
- WHEN the router receives model selector `"gpt-4o-mini"`
- THEN the router MUST dispatch to the default `"openai"` provider with model `"gpt-4o-mini"`

---

### Requirement: Formal Routing Spec — Classification Contract

The formal spec MUST define the classification contract:

1. When `query_classification.enabled = false`, classification MUST return no hint.
2. When enabled with zero rules, classification MUST return no hint.
3. Rules MUST be evaluated in descending `priority` order (highest first).
4. For each rule, length constraints (`min_length`, `max_length`) MUST be checked first. If
   the message length is outside the constraints, the rule MUST be skipped.
5. After length checks, the rule matches if ANY keyword matches (case-insensitive substring)
   OR ANY pattern matches (case-sensitive substring).
6. The first matching rule's `hint` MUST be returned. Subsequent rules MUST NOT be evaluated
   after a match.
7. If no rule matches, classification MUST return no hint.

#### Scenario: Classification disabled returns no hint

- GIVEN `query_classification.enabled = false`
- WHEN a user sends any message
- THEN classification MUST return no hint
- AND the system MUST use the default model

#### Scenario: Enabled with zero rules returns no hint

- GIVEN `query_classification.enabled = true` with empty `rules`
- WHEN a user sends any message
- THEN classification MUST return no hint

#### Scenario: Priority ordering determines evaluation order

- GIVEN two rules:
  - Rule A: `hint = "code"`, `priority = 10`, `keywords = ["debug"]`
  - Rule B: `hint = "reasoning"`, `priority = 20`, `keywords = ["debug"]`
- WHEN a user sends `"help me debug this"`
- THEN classification MUST return hint `"reasoning"` (Rule B, higher priority)
- AND Rule A MUST NOT be evaluated after Rule B matches

#### Scenario: Keyword matching is case-insensitive

- GIVEN a rule with `keywords = ["error"]`
- WHEN a user sends `"I got an ERROR in my code"`
- THEN the rule MUST match

#### Scenario: Pattern matching is case-sensitive

- GIVEN a rule with `patterns = ["fn "]`
- WHEN a user sends `"FN something"`
- THEN the rule MUST NOT match
- AND a message `"fn main()"` MUST match

#### Scenario: Length constraints gate rule evaluation

- GIVEN a rule with `min_length = 100` and `keywords = ["explain"]`
- WHEN a user sends `"explain this"` (12 characters)
- THEN the rule MUST be skipped due to length constraint
- AND keywords MUST NOT be evaluated

#### Scenario: No rule matches returns no hint

- GIVEN classification enabled with rules that match `"code"` keywords
- WHEN a user sends `"what is the weather?"`
- THEN classification MUST return no hint
- AND the system MUST use the default model

---

### Requirement: Formal Routing Spec — Fallback Behavior Contract

The formal spec MUST define fallback behavior:

1. **Unknown hint**: When a hint does not match any configured route, the system MUST use the
   default provider. The system MUST log a warning (per the Silent Failure Fix requirement).
2. **No classification match**: When classification is enabled but no rule matches, the system
   MUST use the default model. No warning is required — this is normal operation.
3. **Classification disabled**: When classification is disabled, the system MUST use the
   configured default model for all requests. No classification evaluation MUST occur.

#### Scenario: Unknown hint falls back with warning

- GIVEN routes for `["fast", "reasoning"]`
- WHEN classification returns hint `"code"` (no matching route)
- THEN the system MUST use the default provider and model
- AND a warning MUST be logged naming the unknown hint

#### Scenario: No classification match uses default model silently

- GIVEN classification enabled with rules
- WHEN a user message matches no rule
- THEN the system MUST use the default model
- AND no warning MUST be logged for the classification miss

#### Scenario: Classification disabled uses default model

- GIVEN `query_classification.enabled = false`
- WHEN any user message is processed
- THEN the system MUST use the default model
- AND no classification logic MUST execute

---

### Requirement: Formal Routing Spec — Image Routing Gating Contract

The formal spec MUST define image routing gating behavior within the model routing context.
This requirement complements the `provider-vision-gating` spec (REQ-6) by specifying how
image routing interacts with the route table.

1. The `vision_model_hint` in `[multimodal]` config MUST resolve against the `[[model_routes]]`
   table.
2. A resolved route MUST have `allow_image_input = true` for image turns to be accepted.
3. If the resolved route has `allow_image_input = false` or the field is unset (defaults to
   `false`), image turns MUST be rejected with `RouteNotImageCapable`.
4. The `allow_image_input` field on `[[model_routes]]` MUST default to `false` (opt-in
   model).

#### Scenario: Vision hint resolves to image-capable route

- GIVEN route `hint = "vision"`, `allow_image_input = true`
- AND `multimodal.vision_model_hint = "vision"`
- WHEN a user sends an image turn
- THEN the router MUST accept and dispatch the image turn to the `"vision"` route's provider

#### Scenario: Vision hint resolves to non-image route

- GIVEN route `hint = "fast"`, `allow_image_input = false`
- AND `multimodal.vision_model_hint = "fast"`
- WHEN a user sends an image turn
- THEN the router MUST reject the image turn with `RouteNotImageCapable`

#### Scenario: Route without explicit allow_image_input defaults to false

- GIVEN route `hint = "default"` with no `allow_image_input` field set
- AND `multimodal.vision_model_hint = "default"`
- WHEN a user sends an image turn
- THEN the router MUST reject the image turn with `RouteNotImageCapable`
- AND the default value of `allow_image_input` MUST be `false`
