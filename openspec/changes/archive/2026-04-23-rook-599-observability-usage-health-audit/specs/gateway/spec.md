# Delta for gateway

## ADDED Requirements

### Requirement: Persisted Append-Only Admin Audit Events

The system MUST persist an append-only admin audit record whenever a supported admin mutation
successfully changes gateway control-plane state.

Supported mutation categories for this slice MUST include:

- account create, update, and delete operations
- pool create, update, and delete operations
- pool membership add and remove operations
- route create, update, and delete operations
- settings update operations

Each audit record MUST be durably stored so that it survives process restart.

Each audit record MUST be append-only: once written, the record MUST NOT be updated in place to alter
its action, subject, actor context, or payload.

Failed validation, authorization, or conflict attempts MAY be excluded from persistence for this slice;
the required scope is successful persisted mutations only.

#### Scenario: successful account mutation writes audit record

- GIVEN an admin request successfully creates or updates an account
- WHEN the mutation is committed
- THEN the system MUST append exactly one persisted audit record for that mutation
- AND the record MUST identify the resource category as `account`
- AND the record MUST identify the mutation action that occurred

#### Scenario: successful pool membership change writes audit record

- GIVEN an admin request successfully adds or removes an account from a pool
- WHEN the membership change is committed
- THEN the system MUST append exactly one persisted audit record for that mutation
- AND the record MUST identify the resource category as `pool_membership`

#### Scenario: failed mutation does not require persisted audit record

- GIVEN an admin mutation request is rejected before any state change is committed
- WHEN the API returns a validation, not-found, or conflict error
- THEN this slice MUST NOT require a persisted audit record for that rejected attempt

### Requirement: Minimal Redacted Audit Payload Semantics

The system MUST store only a minimal admin-safe audit payload for this slice.

Each persisted audit record MUST include enough metadata to answer who acted within the available
request context, what mutation occurred, what resource category was affected, which resource identity
was targeted, and when the change was committed.

The stored payload MUST be redacted and bounded. It MUST NOT persist raw secrets, credentials,
authorization headers, API keys, bearer tokens, session cookies, or other sensitive values from the
request or resulting resource state.

When a mutation involves fields that are secret-bearing or operationally sensitive, the audit payload
MUST either omit those fields entirely or persist only an explicit redacted marker rather than the raw
value.

The audit payload SHOULD avoid storing full before/after resource snapshots when narrower changed-field
or identifier-oriented metadata is sufficient.

#### Scenario: account secret fields are excluded from audit payload

- GIVEN an admin request creates or updates an account with an `api_key` or other credential material
- WHEN the audit record is persisted
- THEN the persisted payload MUST NOT contain the raw credential value
- AND the record MUST preserve only redacted or non-secret mutation metadata

#### Scenario: auth transport secrets are excluded from audit payload

- GIVEN an authenticated admin mutation request includes authorization headers or cookies
- WHEN the audit record is persisted
- THEN the persisted payload MUST NOT contain those raw header or cookie values

#### Scenario: settings audit payload remains bounded

- GIVEN an admin request updates settings
- WHEN the audit record is persisted
- THEN the payload MUST capture only the minimal settings mutation metadata needed for auditability
- AND the payload MUST NOT expand into an unbounded full-observability document

### Requirement: Admin Audit Retrieval Endpoint

The system MUST provide a bounded admin read surface for retrieving recent persisted audit events.

If this slice exposes audit retrieval, the endpoint MUST be read-only and admin-scoped.

Audit retrieval MUST return persisted audit events in reverse chronological order, newest first.

Each returned audit item MUST preserve the same redaction guarantees as the stored record.

The retrieval contract MAY be limited to recent events only and MAY omit advanced filtering, full-text
search, historical analytics, or retention management.

#### Scenario: audit retrieval returns newest records first

- GIVEN multiple persisted audit records exist for prior admin mutations
- WHEN an admin client requests the audit trail
- THEN the response status MUST be `200 OK`
- AND the response body MUST contain recent audit records ordered newest first

#### Scenario: audit retrieval returns redacted records only

- GIVEN persisted audit records exist for secret-adjacent mutations
- WHEN an admin client requests the audit trail
- THEN the response MUST NOT reveal raw secrets or credentials
- AND each returned item MUST match the redacted audit payload contract

## MODIFIED Requirements

### Requirement: Health Account List Endpoint

The system MUST expose `GET /api/health/accounts`.

The response MUST be a JSON array of `HealthAccountView` records representing runtime health state
for known accounts.

For this slice, health data SHALL remain runtime-scoped and in-memory only. It MUST reflect current
process state and MUST NOT imply durable historical health storage, automatic health snapshots, or
 persisted health history.

When an account exists but has never been probed, its health status MUST be `"unknown"`.

(Previously: For M1, health data is runtime-scoped and in-memory only. It MUST reflect current
process state and MUST NOT imply durable historical health storage.)

#### Scenario: health account list remains runtime-only after audit slice

- GIVEN the audit slice has been added
- WHEN a client requests `GET /api/health/accounts`
- THEN the response MUST still represent current runtime health state only
- AND the response MUST NOT claim or require durable health history

### Requirement: Health Summary Endpoint

The system MUST expose `GET /api/health/summary`.

The response MUST be a `HealthSummaryView` object summarizing known account health state for the
current runtime.

The summary MUST include counts for `healthy`, `degraded`, `unhealthy`, `unknown`, and `total`.

For this slice, the summary MUST remain a current-state runtime view and MUST NOT be reinterpreted as
historical health reporting.

(Previously: The response MUST be a `HealthSummaryView` object summarizing known account health state
for the current runtime.)

#### Scenario: health summary does not become historical reporting

- GIVEN persisted admin audit events exist
- WHEN a client requests `GET /api/health/summary`
- THEN the response MUST still summarize current runtime health only
- AND the response MUST NOT include or imply persisted health history

### Requirement: Usage Placeholder Endpoint

The system MUST expose `GET /api/usage`.

Because no real usage or cost-accounting backend exists in M1, this endpoint MUST return a stable
placeholder response using `UsageStatusView` with `available: false`.

The endpoint MUST NOT invent fake usage totals, provider billing details, quota consumption, token
accounting, or analytics summaries.

This audit slice MUST preserve that placeholder behavior unchanged unless a separate change adds a real
usage ledger and corresponding specification updates.

(Previously: The endpoint MUST NOT invent fake usage totals or provider billing details.)

#### Scenario: usage endpoint remains placeholder after audit slice

- GIVEN persisted admin audit events exist in the system
- WHEN a client requests `GET /api/usage`
- THEN the response MUST still equal the documented placeholder contract
- AND `available` MUST be `false`
- AND the response MUST NOT claim real usage analytics or accounting
