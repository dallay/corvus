package com.profiletailors.agent.core

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertNull
import kotlin.test.assertTrue

class CoreContractsTest {
  @Test
  fun `should apply invocation defaults`() {
    val invocation = CoreInvocation(prompt = "hello")

    assertEquals("hello", invocation.prompt)
    assertEquals(null, invocation.sessionId)
    assertTrue(invocation.metadata.isEmpty())
    assertEquals(null, invocation.timeoutMs)
  }

  @Test
  fun `should model success and failure results`() {
    val success = CoreResult.Success(CoreOutput(text = "ok", transport = "rust-cli"))
    val failure = CoreResult.Failure(message = "boom", recoverable = true)

    assertIs<CoreResult.Success>(success)
    assertIs<CoreResult.Failure>(failure)
  }

  @Test
  fun `should expose canonical onboarding primitives`() {
    assertEquals(
      listOf(
        SurfaceId.CLI_RUNTIME,
        SurfaceId.WEB_DASHBOARD,
        SurfaceId.WEB_CHAT,
        SurfaceId.COMPOSEAPP_MOBILE,
      ),
      SurfaceId.entries,
    )

    assertEquals(
      listOf(TrustMode.HOST_TRUSTED, TrustMode.HTTP_PAIRED, TrustMode.BRIDGE_LINKED),
      TrustMode.entries,
    )

    assertEquals(
      listOf(TransportMode.DIRECT, TransportMode.HTTP_GATEWAY, TransportMode.CLI_BRIDGE),
      TransportMode.entries,
    )

    assertEquals(
      listOf(
        RecoveryKind.RUNTIME_UNAVAILABLE,
        RecoveryKind.TRANSPORT_UNAVAILABLE,
        RecoveryKind.TRUST_INPUT_INVALID,
        RecoveryKind.TRUST_INPUT_EXPIRED,
        RecoveryKind.CREDENTIAL_MISSING,
        RecoveryKind.CREDENTIAL_INVALID,
        RecoveryKind.PAIRED_BUT_NOT_CONNECTED,
        RecoveryKind.LINKED_BUT_NOT_SESSION_READY,
        RecoveryKind.SESSION_UNAVAILABLE,
        RecoveryKind.ENVIRONMENT_UNSUPPORTED,
      ),
      RecoveryKind.entries,
    )
  }

  @Test
  fun `should distinguish ready states from session states`() {
    val readyState =
      OnboardingState(
        surfaceId = SurfaceId.WEB_DASHBOARD,
        state = OnboardingState.Status.READY,
        trustMode = TrustMode.HTTP_PAIRED,
        transportMode = TransportMode.HTTP_GATEWAY,
        canRetry = true,
        canResume = true,
      )
    val sessionPendingState =
      readyState.copy(
        surfaceId = SurfaceId.WEB_CHAT,
        state = OnboardingState.Status.SESSION_PENDING,
      )
    val sessionReadyState = sessionPendingState.copy(state = OnboardingState.Status.SESSION_READY)

    assertTrue(readyState.isReadyForPrimaryTask)
    assertFalse(readyState.requiresSessionEntry)
    assertTrue(sessionPendingState.isReadyForPrimaryTask)
    assertTrue(sessionPendingState.requiresSessionEntry)
    assertTrue(sessionReadyState.isReadyForPrimaryTask)
    assertTrue(sessionReadyState.requiresSessionEntry)
  }

  @Test
  fun `should require recovery kind for blocked onboarding states`() {
    val blockedState =
      OnboardingState(
        surfaceId = SurfaceId.WEB_CHAT,
        state = OnboardingState.Status.BLOCKED,
        trustMode = TrustMode.HTTP_PAIRED,
        transportMode = TransportMode.HTTP_GATEWAY,
        recoveryKind = RecoveryKind.CREDENTIAL_INVALID,
        canRetry = true,
        canResume = false,
      )

    assertEquals(OnboardingState.Status.BLOCKED, blockedState.state)
    assertEquals(RecoveryKind.CREDENTIAL_INVALID, blockedState.recoveryKind)
    assertTrue(blockedState.canRetry)
    assertFalse(blockedState.canResume)
  }

  @Test
  fun `should keep bridge snapshots scoped to the mobile surface contract`() {
    val snapshot =
      BridgeLinkSnapshot(runtimeAvailable = true, linkEstablished = true, sessionCapable = true)

    val error =
      assertFailsWith<IllegalArgumentException> {
        snapshot.toOnboardingState(surfaceId = SurfaceId.WEB_CHAT)
      }

    assertTrue(error.message.orEmpty().contains("COMPOSEAPP_MOBILE"))
  }

  @Test
  fun `should enforce uuid session identifiers for runtime-backed mobile sessions`() {
    val sessionId = SessionId("550e8400-e29b-41d4-a716-446655440000")

    assertEquals("550e8400-e29b-41d4-a716-446655440000", sessionId.value)

    assertFailsWith<IllegalArgumentException> { SessionId("not-a-uuid") }
  }

  @Test
  fun `should expose readiness capabilities and session restore semantics`() {
    val snapshot =
      MobileRuntimeReadinessSnapshot(
        runtimeAvailable = true,
        linkEstablished = true,
        sessionCapable = true,
        activeSessionId = SessionId("550e8400-e29b-41d4-a716-446655440000"),
        capabilities =
          MobileRuntimeCapabilities(
            streamingResponses = false,
            resumableSessionList = true,
            approvalRequests = true,
          ),
      )

    val state = snapshot.toOnboardingState()

    assertEquals(OnboardingState.Status.SESSION_READY, state.state)
    assertTrue(state.canResume)
    assertTrue(snapshot.capabilities.resumableSessionList)
    assertTrue(snapshot.capabilities.approvalRequests)
    assertFalse(snapshot.capabilities.streamingResponses)
  }

  @Test
  fun `should model runtime-backed approvals as explicit session-scoped decisions`() {
    val sessionId = SessionId("550e8400-e29b-41d4-a716-446655440000")
    val request =
      MobileApprovalRequest(
        id = "approval-1",
        sessionId = sessionId,
        toolLabel = "shell",
        reason = "Run a runtime-gated command",
      )
    val pending = MobileRuntimeEvent.ApprovalPending(request)
    val resolved =
      MobileRuntimeTurnResult(
        sessionId = sessionId,
        events = listOf(MobileRuntimeEvent.AssistantMessage(sessionId, "approved")),
      )

    assertEquals(sessionId, request.sessionId)
    assertEquals("shell", request.toolLabel)
    assertIs<MobileRuntimeEvent.ApprovalPending>(pending)
    assertEquals(sessionId, resolved.sessionId)
    assertEquals(1, resolved.events.size)
    assertEquals(MobileApprovalDecision.APPROVE, MobileApprovalDecision.valueOf("APPROVE"))
    assertEquals(MobileApprovalDecision.DENY, MobileApprovalDecision.valueOf("DENY"))
  }

  @Test
  fun `should persist linked metadata and active session identity through shared boundary`() {
    val persistence = InMemoryMobileRuntimePersistence()
    val metadata =
      LinkedRuntimeMetadata(
        targetId = "android-local-runtime",
        transportMode = TransportMode.CLI_BRIDGE,
        trustMode = TrustMode.BRIDGE_LINKED,
      )
    val sessionId = SessionId("550e8400-e29b-41d4-a716-446655440000")

    persistence.saveLinkedRuntimeMetadata(metadata)
    persistence.saveActiveSessionId(sessionId)

    assertEquals(metadata, persistence.readLinkedRuntimeMetadata())
    assertEquals(sessionId, persistence.readActiveSessionId())

    persistence.clearLinkedRuntimeMetadata()
    persistence.clearActiveSessionId()

    assertNull(persistence.readLinkedRuntimeMetadata())
    assertNull(persistence.readActiveSessionId())
  }

  // ======= Client-First Connection Model Tests (Task 1.1 - RED) =======

  @Test
  fun `should require explicit connection method in runtime connection target`() {
    // Client-first model: targets MUST declare their connection method explicitly
    // Current implementation defaults to CLI_BRIDGE which is invalid for client-first
    val targetWithEndpoint =
      RuntimeConnectionTarget(
        id = "remote-runtime-1",
        label = "Remote Corvus",
        method = RuntimeConnectionMethod.ENDPOINT_URL,
        endpointUrl = "https://corvus.example.com",
      )
    val targetWithCompanion =
      RuntimeConnectionTarget(
        id = "companion-1",
        label = "Trusted Companion",
        method = RuntimeConnectionMethod.TRUSTED_COMPANION,
      )

    assertEquals(RuntimeConnectionMethod.ENDPOINT_URL, targetWithEndpoint.method)
    assertEquals("https://corvus.example.com", targetWithEndpoint.endpointUrl)
    assertEquals(RuntimeConnectionMethod.TRUSTED_COMPANION, targetWithCompanion.method)
    assertNull(targetWithCompanion.endpointUrl)
  }

  @Test
  fun `should gate chat behind connection target, trust, and reachability`() {
    // Chat MUST NOT be accessible unless:
    // 1. A connection target is explicitly selected
    // 2. Trust/auth is established
    // 3. Transport is reachable

    // Without target: onboarding required
    val noTargetSnapshot =
      MobileRuntimeReadinessSnapshot(
        runtimeAvailable = false,
        linkEstablished = false,
        sessionCapable = false,
      )
    val noTargetState = noTargetSnapshot.toOnboardingState()
    assertTrue(noTargetState.state == OnboardingState.Status.BLOCKED)

    // With target but no trust: trust pending
    val targetSelectedSnapshot =
      MobileRuntimeReadinessSnapshot(
        runtimeAvailable = true,
        linkEstablished = false, // No trust established yet
        sessionCapable = false,
      )
    val trustPendingState = targetSelectedSnapshot.toOnboardingState()
    assertTrue(trustPendingState.state == OnboardingState.Status.TRUST_PENDING)
    assertFalse(trustPendingState.isReadyForPrimaryTask)

    // With target and trust but not reachable: blocked
    val notReachableSnapshot =
      MobileRuntimeReadinessSnapshot(
        runtimeAvailable = false, // Not reachable
        linkEstablished = true, // Trust established
        sessionCapable = false,
      )
    val notReachableState = notReachableSnapshot.toOnboardingState()
    assertTrue(notReachableState.state == OnboardingState.Status.BLOCKED)
  }

  @Test
  fun `should expose trust state with explicit pairing and companion semantics`() {
    // Client-first trust states: not just BRIDGE_LINKED, but explicit pairing/companion
    val trustState = RuntimeTrustState(established = true, requiresPairingOrAuth = true)

    assertTrue(trustState.established)
    assertTrue(trustState.requiresPairingOrAuth)

    val untrustedState = RuntimeTrustState(established = false, requiresPairingOrAuth = true)
    assertFalse(untrustedState.established)
  }

  @Test
  fun `should model readiness with explicit target and supported methods`() {
    // Readiness snapshot must include:
    // - Selected target (if any)
    // - Trust state
    // - Transport reachability
    // - Supported connection methods for this surface
    val snapshot =
      RuntimeConnectionReadinessSnapshot(
        target =
          RuntimeConnectionTarget(
            id = "remote-runtime",
            label = "My Corvus",
            method = RuntimeConnectionMethod.ENDPOINT_URL,
            endpointUrl = "https://corvus.example.com",
          ),
        trustState = RuntimeTrustState(established = true, requiresPairingOrAuth = false),
        transportReachable = true,
        sessionCapable = true,
        activeSessionId = null,
        supportedMethods = setOf(RuntimeConnectionMethod.ENDPOINT_URL),
      )

    assertEquals(RuntimeConnectionMethod.ENDPOINT_URL, snapshot.target?.method)
    assertTrue(snapshot.trustState.established)
    assertTrue(snapshot.transportReachable)
    assertTrue(snapshot.sessionCapable)
    assertEquals(1, snapshot.supportedMethods.size)
  }

  @Test
  fun `should fail chat entry when no connection target is configured`() {
    // Mobile surfaces MUST NOT allow chat when no target is selected
    val noTargetReadiness =
      RuntimeConnectionReadinessSnapshot(
        target = null,
        trustState = RuntimeTrustState(established = false, requiresPairingOrAuth = false),
        transportReachable = false,
        sessionCapable = false,
        activeSessionId = null,
        supportedMethods = emptySet(),
      )

    // This should block chat access
    assertTrue(noTargetReadiness.target == null)
    assertFalse(noTargetReadiness.isReadyForChat)
  }

  private class InMemoryMobileRuntimePersistence : MobileRuntimePersistence {
    private var metadata: LinkedRuntimeMetadata? = null
    private var activeSessionId: SessionId? = null

    override fun readLinkedRuntimeMetadata(): LinkedRuntimeMetadata? = metadata

    override fun saveLinkedRuntimeMetadata(metadata: LinkedRuntimeMetadata) {
      this.metadata = metadata
    }

    override fun clearLinkedRuntimeMetadata() {
      metadata = null
    }

    override fun readActiveSessionId(): SessionId? = activeSessionId

    override fun saveActiveSessionId(sessionId: SessionId) {
      this.activeSessionId = sessionId
    }

    override fun clearActiveSessionId() {
      activeSessionId = null
    }
  }
}
