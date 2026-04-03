# Corvus Product Glossary

> Canonical product terminology for all Corvus surfaces.
> Source of truth: [`terms.json`](./terms.json). This document is the human-readable reference.

## Terms

### Agent

**Canonical**: Corvus Agent
**Definition**: The AI agent powered by the Corvus runtime. The product identity for the assistant
personality across all surfaces.
**Context**: Always use "Corvus Agent" (capitalized) in user-facing text. Never refer to the agent
generically as a bot or assistant.
**Anti-terms**: assistant, bot, AI, model (too generic; loses product identity)

### Session

**Canonical**: Session
**Definition**: A bounded interaction context between a user and the Corvus Agent, identified by a
UUID v4.
**Context**: Used across all session-capable surfaces. A session groups messages, tool calls, and
memory operations into a single interaction unit.
**Anti-terms**: conversation, thread, chat (as noun — "chat" refers to the surface/action, not the
session)

### Chat

**Canonical**: Chat
**Definition**: The interactive messaging interface where users communicate with the Corvus Agent.
**Context**: Refers to the action and surface for real-time messaging. Do not use "chat" as a noun
synonym for "session".
**Anti-terms**: message, conversation (use "session" for the bounded interaction context)

### Surface

**Canonical**: Surface
**Definition**: A client application in the Corvus ecosystem that presents capabilities to users or
operators.
**Context**: Corvus-specific term for any client interface — web, mobile, CLI, docs, or marketing.
Each surface has a defined role and transport.
**Anti-terms**: app, client, frontend, interface (too generic; "surface" carries Corvus-specific
semantics)

### Pairing

**Canonical**: Pair / Pairing
**Definition**: The one-time trust exchange where a surface receives credentials to communicate with
the Corvus runtime.
**Context**: Web surfaces exchange a pairing code for a bearer token. Mobile surfaces perform local
linking, but the *user-facing term* is always "pairing" — the glossary governs the user-facing term,
not the technical mechanism.
**Anti-terms**: link, linking (mobile previously used "link" — this is now deprecated), connect,
bind, associate

### Trust

**Canonical**: Trust
**Definition**: The onboarding step where a surface is authorized to interact with the Corvus
runtime.
**Context**: Trust is established during pairing. Once trusted, the surface can communicate over its
assigned transport.
**Anti-terms**: authorize, approve (too procedural; "trust" conveys the relationship model)

### Runtime

**Canonical**: Runtime
**Definition**: The Rust-based Corvus agent execution environment that processes agent loops, tool
calls, and memory operations.
**Context**: The core backend process. Surfaces never access the runtime directly except the CLI;
all
others go through a gateway or bridge.
**Anti-terms**: server, backend, engine, daemon (too generic; "runtime" is the Corvus-specific term)

### Gateway

**Canonical**: Gateway
**Definition**: The HTTP API bridge between web surfaces and the Corvus runtime.
**Context**: Web clients (chat, dashboard) communicate exclusively through the gateway. Exposes a
client-safe subset of runtime capabilities over REST/WebSocket.
**Anti-terms**: API, proxy, server, relay (too generic or misleading)

### Bridge

**Canonical**: Bridge
**Definition**: The process-level connection between mobile surfaces and the Corvus runtime via
stdin/stdout.
**Context**: Mobile clients use the RustCliBridge for runtime communication. The bridge spawns the
runtime as a subprocess.
**Anti-terms**: connector, adapter, wrapper (too generic)

### Onboarding

**Canonical**: Onboarding
**Definition**: The first-run setup flow where a surface discovers the runtime, establishes trust,
and reaches a ready state.
**Context**: All onboarding-capable surfaces implement the canonical onboarding steps. The flow
varies by transport but achieves the same outcome.
**Anti-terms**: setup, wizard, first-run, registration (too generic or implies different UX
patterns)

### Tool

**Canonical**: Tool
**Definition**: An MCP-registered capability that the Corvus Agent can invoke to perform actions on
behalf of the user.
**Context**: Tools are registered in the MCP runtime and may require user approval before execution.
Displayed in chat and dashboard surfaces.
**Anti-terms**: function, action, command, skill (overloaded terms from other ecosystems)

### Memory

**Canonical**: Memory
**Definition**: The agent's persistent knowledge system powered by Cerebro, providing long-term
context across sessions.
**Context**: Memory is managed through the Cerebro module using MCP tools. Accessible from chat,
dashboard, and CLI surfaces.
**Anti-terms**: context, history, recall, knowledge base (too vague or implies different systems)

### Operator

**Canonical**: Operator
**Definition**: A human administrator who manages the Corvus runtime, configures surfaces, and
monitors system health.
**Context**: Operators interact through the CLI and dashboard surfaces. They have elevated
privileges
compared to end-users.
**Anti-terms**: admin, administrator (too generic; "operator" conveys the runtime management role)

## Maintenance

This glossary is maintained under the governance process defined in
[`GOVERNANCE.md`](./GOVERNANCE.md). The machine-readable source of truth is
[`terms.json`](./terms.json). All changes to terminology must follow the term lifecycle process.
