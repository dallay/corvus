# Delta for Dashboard

## ADDED Requirements

### Requirement: Dashboard Onboarding Boundary

The dashboard specification SHALL remain the operator-specific source of truth for dashboard
activation behavior, while aligning its user-visible sequence and terminology to the shared
onboarding specification.

#### Scenario: Dashboard activation remains an operator slice of shared onboarding

- GIVEN a user reaches the dashboard activation portion of onboarding
- WHEN the dashboard flow is evaluated
- THEN the dashboard spec MUST govern the operator-specific activation behavior
- AND the shared onboarding spec MUST govern the cross-surface sequence and terminology used around
  that slice.

#### Scenario: Dashboard recovery language matches shared taxonomy

- GIVEN the dashboard activation flow diagnoses a blocked or incomplete state
- WHEN guidance is shown to the user
- THEN the diagnosis MUST map to the shared onboarding recovery taxonomy
- AND the dashboard MAY provide operator-specific commands or next actions within that taxonomy.

## MODIFIED Requirements

### Requirement: Accepted-Path Activation Guidance

If the user accepts dashboard activation, the system SHALL provide a compact operator activation
guide that fits into the canonical onboarding model: confirm runtime availability, complete HTTP
pairing to acquire a bearer token when required, connect to the gateway, and confirm dashboard-ready
state.

(Previously: If the user accepts activation, the system shall provide a compact activation guide
that includes: dashboard URL to open, gateway status expectation, pairing instructions, and optional
browser-open attempt.)

#### Scenario: Accepted activation uses canonical terminology and sequence

- GIVEN interactive onboarding offers dashboard activation
- WHEN the user accepts
- THEN the activation guidance MUST use the shared terms `pairing`, `pairing code`, `bearer token`,
  and `connect to gateway` where applicable
- AND the guidance MUST present dashboard activation as the operator-specific continuation of the
  canonical onboarding sequence.

### Requirement: Deterministic Diagnosis and Fallback Commands

For accepted dashboard activation flow, the system SHALL classify activation readiness or failure
using the shared onboarding recovery taxonomy before presenting operator-specific fallback commands.

(Previously: For accepted activation flow, the system shall classify activation readiness/failure
into deterministic local states and provide exact manual fallback commands for each state.)

#### Scenario: Dashboard diagnosis maps to shared recovery states

- GIVEN the dashboard activation flow detects a failure
- WHEN the failure is reported
- THEN the diagnosis MUST map to one of the shared onboarding recovery states applicable to the
  dashboard
- AND the printed fallback commands MUST remain copy-paste ready for the operator.
