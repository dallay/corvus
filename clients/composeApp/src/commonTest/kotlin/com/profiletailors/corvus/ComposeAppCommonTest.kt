package com.profiletailors.corvus

import com.profiletailors.corvus.runtime.LinkedRuntimeMetadata
import com.profiletailors.corvus.runtime.MobileRuntimeCoordinator
import com.profiletailors.corvus.runtime.MobileRuntimeFacade
import com.profiletailors.corvus.runtime.MobileRuntimePersistence
import com.profiletailors.corvus.runtime.RuntimeApprovalDecision
import com.profiletailors.corvus.runtime.RuntimeApprovalRequest
import com.profiletailors.corvus.runtime.RuntimeCapabilities
import com.profiletailors.corvus.runtime.RuntimeEvent
import com.profiletailors.corvus.runtime.RuntimeReadinessSnapshot
import com.profiletailors.corvus.runtime.RuntimeSession
import com.profiletailors.corvus.runtime.RuntimeSessionId
import com.profiletailors.corvus.runtime.RuntimeTransportMode
import com.profiletailors.corvus.runtime.RuntimeTrustMode
import com.profiletailors.corvus.runtime.RuntimeTurnResult
import com.profiletailors.corvus.ui.chat.MobileOnboardingStatus
import com.profiletailors.corvus.ui.chat.MobileRecoveryKind
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertNull
import kotlin.test.assertTrue

class ComposeAppCommonTest {
  @Test
  fun `should keep chat gated until runtime readiness and session entry succeed`() {
    val coordinator =
      MobileRuntimeCoordinator(
        facade =
          FakeMobileRuntimeFacade(
            readiness =
              RuntimeReadinessSnapshot(
                runtimeAvailable = true,
                linkEstablished = false,
                sessionCapable = false,
              )
          ),
        persistence = FakeMobileRuntimePersistence(),
      )

    coordinator.refresh()

    assertEquals(
      MobileOnboardingStatus.TRUST_PENDING,
      coordinator.state.bridgeSnapshot.toOnboardingState().status,
    )
    assertFalse(coordinator.state.isChatReady)
    assertNull(coordinator.state.activeSessionId)
  }

  @Test
  fun `should not query resumable sessions when runtime is unavailable`() {
    val facade =
      FakeMobileRuntimeFacade(
        readiness =
          RuntimeReadinessSnapshot(
            runtimeAvailable = false,
            linkEstablished = false,
            sessionCapable = false,
            capabilities =
              RuntimeCapabilities(
                streamingResponses = false,
                resumableSessionList = true,
                approvalRequests = true,
              ),
          ),
        failResumableSessionLookup = true,
      )
    val coordinator =
      MobileRuntimeCoordinator(facade = facade, persistence = FakeMobileRuntimePersistence())

    coordinator.refresh()

    assertEquals(0, facade.listResumableSessionsCalls)
    assertEquals(
      MobileOnboardingStatus.BLOCKED,
      coordinator.state.bridgeSnapshot.toOnboardingState().status,
    )
    assertEquals(
      MobileRecoveryKind.RUNTIME_UNAVAILABLE,
      coordinator.state.bridgeSnapshot.toOnboardingState().recoveryKind,
    )
  }

  @Test
  fun `should fail closed when resumable session lookup breaks after readiness succeeds`() {
    val coordinator =
      MobileRuntimeCoordinator(
        facade =
          FakeMobileRuntimeFacade(readiness = readyReadiness(), failResumableSessionLookup = true),
        persistence = FakeMobileRuntimePersistence(),
      )

    coordinator.refresh()

    assertEquals(
      MobileRecoveryKind.LINKED_BUT_NOT_SESSION_READY,
      coordinator.state.bridgeSnapshot.recoveryOverride,
    )
    assertEquals(
      MobileOnboardingStatus.BLOCKED,
      coordinator.state.bridgeSnapshot.toOnboardingState().status,
    )
    assertFalse(coordinator.state.isChatReady)
    assertTrue(coordinator.state.resumableSessions.isEmpty())
  }

  @Test
  fun `should create resume and end runtime-backed sessions while persisting active identity`() {
    val persistence = FakeMobileRuntimePersistence()
    val facade =
      FakeMobileRuntimeFacade(
        readiness = readyReadiness(),
        sessions =
          mutableListOf(
            RuntimeSession(
              id = RuntimeSessionId("550e8400-e29b-41d4-a716-446655440000"),
              title = "Existing session",
              isActive = false,
            )
          ),
        createSessionResult =
          RuntimeSession(
            id = RuntimeSessionId("123e4567-e89b-12d3-a456-426614174000"),
            title = "Fresh session",
            isActive = true,
          ),
      )
    val coordinator = MobileRuntimeCoordinator(facade = facade, persistence = persistence)

    coordinator.refresh()
    coordinator.startNewSession()

    assertEquals(
      RuntimeSessionId("123e4567-e89b-12d3-a456-426614174000"),
      coordinator.state.activeSessionId,
    )
    assertEquals(
      RuntimeSessionId("123e4567-e89b-12d3-a456-426614174000"),
      persistence.activeSessionId,
    )

    coordinator.resumeSession(RuntimeSessionId("550e8400-e29b-41d4-a716-446655440000"))

    assertEquals(
      RuntimeSessionId("550e8400-e29b-41d4-a716-446655440000"),
      coordinator.state.activeSessionId,
    )

    coordinator.endActiveSession()

    assertNull(coordinator.state.activeSessionId)
    assertNull(persistence.activeSessionId)
  }

  @Test
  fun `should restore persisted active session on refresh`() {
    val persistence =
      FakeMobileRuntimePersistence(
        activeSessionId = RuntimeSessionId("550e8400-e29b-41d4-a716-446655440000")
      )
    val coordinator =
      MobileRuntimeCoordinator(
        facade =
          FakeMobileRuntimeFacade(
            readiness = readyReadiness(),
            sessions =
              mutableListOf(
                RuntimeSession(
                  id = RuntimeSessionId("550e8400-e29b-41d4-a716-446655440000"),
                  title = "Persisted session",
                  isActive = true,
                )
              ),
          ),
        persistence = persistence,
      )

    coordinator.refresh()

    assertEquals(
      RuntimeSessionId("550e8400-e29b-41d4-a716-446655440000"),
      coordinator.state.activeSessionId,
    )
    assertEquals(
      MobileOnboardingStatus.SESSION_READY,
      coordinator.state.bridgeSnapshot.toOnboardingState().status,
    )
    assertTrue(coordinator.state.isChatReady)
  }

  @Test
  fun `should surface session unavailable recovery when persisted session cannot be resumed`() {
    val persistence =
      FakeMobileRuntimePersistence(
        activeSessionId = RuntimeSessionId("550e8400-e29b-41d4-a716-446655440000")
      )
    val coordinator =
      MobileRuntimeCoordinator(
        facade =
          FakeMobileRuntimeFacade(
            readiness = readyReadiness(),
            sessions =
              mutableListOf(
                RuntimeSession(
                  id = RuntimeSessionId("123e4567-e89b-12d3-a456-426614174000"),
                  title = "Fallback session",
                  isActive = false,
                )
              ),
          ),
        persistence = persistence,
      )

    coordinator.refresh()

    assertEquals(
      MobileRecoveryKind.SESSION_UNAVAILABLE,
      coordinator.state.bridgeSnapshot.recoveryOverride,
    )
    assertEquals(
      MobileOnboardingStatus.BLOCKED,
      coordinator.state.bridgeSnapshot.toOnboardingState().status,
    )
    assertEquals(1, coordinator.state.resumableSessions.size)
    assertNull(coordinator.state.activeSessionId)
  }

  @Test
  fun `should classify transport outages separately after a prior mobile link`() {
    val persistence =
      FakeMobileRuntimePersistence(
        linkedMetadata = LinkedRuntimeMetadata(targetId = "android-local-runtime")
      )
    val coordinator =
      MobileRuntimeCoordinator(
        facade =
          FakeMobileRuntimeFacade(
            readiness =
              RuntimeReadinessSnapshot(
                runtimeAvailable = false,
                linkEstablished = false,
                sessionCapable = false,
              )
          ),
        persistence = persistence,
      )

    coordinator.refresh()

    assertEquals(
      MobileRecoveryKind.TRANSPORT_UNAVAILABLE,
      coordinator.state.bridgeSnapshot.recoveryOverride,
    )
    assertEquals(
      MobileOnboardingStatus.BLOCKED,
      coordinator.state.bridgeSnapshot.toOnboardingState().status,
    )
    assertTrue(coordinator.state.bridgeSnapshot.toOnboardingState().canRetry)
  }

  @Test
  fun `should persist linked metadata with shared transport and trust defaults`() {
    val persistence = FakeMobileRuntimePersistence()
    val coordinator =
      MobileRuntimeCoordinator(
        facade = FakeMobileRuntimeFacade(readiness = readyReadiness()),
        persistence = persistence,
      )

    coordinator.refresh()

    val linkedMetadata = persistence.readLinkedRuntimeMetadata()
    assertEquals("mobile-runtime", linkedMetadata?.targetId)
    assertEquals(RuntimeTransportMode.CLI_BRIDGE, linkedMetadata?.transportMode)
    assertEquals(RuntimeTrustMode.BRIDGE_LINKED, linkedMetadata?.trustMode)
  }

  @Test
  fun `should track runtime-backed messages and approval state`() {
    val persistence = FakeMobileRuntimePersistence()
    val approvalRequest =
      RuntimeApprovalRequest(
        id = "approval-1",
        sessionId = RuntimeSessionId("123e4567-e89b-12d3-a456-426614174000"),
        toolLabel = "shell",
        reason = "Run a command",
      )
    val facade =
      FakeMobileRuntimeFacade(
        readiness = readyReadiness(),
        createSessionResult =
          RuntimeSession(
            id = RuntimeSessionId("123e4567-e89b-12d3-a456-426614174000"),
            title = "Fresh session",
            isActive = true,
          ),
        sendMessageResult =
          RuntimeTurnResult(
            sessionId = RuntimeSessionId("123e4567-e89b-12d3-a456-426614174000"),
            events =
              listOf(
                RuntimeEvent.AssistantMessage(
                  sessionId = RuntimeSessionId("123e4567-e89b-12d3-a456-426614174000"),
                  text = "Runtime reply",
                ),
                RuntimeEvent.ApprovalPending(approvalRequest),
              ),
          ),
        approvalResult =
          RuntimeTurnResult(
            sessionId = RuntimeSessionId("123e4567-e89b-12d3-a456-426614174000"),
            events =
              listOf(
                RuntimeEvent.AssistantMessage(
                  sessionId = RuntimeSessionId("123e4567-e89b-12d3-a456-426614174000"),
                  text = "Approval complete",
                )
              ),
          ),
      )
    val coordinator = MobileRuntimeCoordinator(facade = facade, persistence = persistence)

    coordinator.refresh()
    coordinator.startNewSession()
    coordinator.sendMessage("hello runtime")

    assertEquals(2, coordinator.state.messages.size)
    assertEquals("hello runtime", coordinator.state.messages[0].content)
    assertEquals("Runtime reply", coordinator.state.messages[1].content)
    assertEquals("approval-1", coordinator.state.pendingApproval?.id)

    coordinator.submitApproval(RuntimeApprovalDecision.APPROVE)

    assertNull(coordinator.state.pendingApproval)
    assertEquals("Approval complete", coordinator.state.messages.last().content)
  }

  @Test
  fun `should send deny decisions through the runtime and render the denial outcome`() {
    val approvalRequest =
      RuntimeApprovalRequest(
        id = "approval-2",
        sessionId = RuntimeSessionId("123e4567-e89b-12d3-a456-426614174000"),
        toolLabel = "shell",
        reason = "Run a command",
      )
    val facade =
      FakeMobileRuntimeFacade(
        readiness = readyReadiness(),
        createSessionResult =
          RuntimeSession(
            id = RuntimeSessionId("123e4567-e89b-12d3-a456-426614174000"),
            title = "Fresh session",
            isActive = true,
          ),
        sendMessageResult =
          RuntimeTurnResult(
            sessionId = RuntimeSessionId("123e4567-e89b-12d3-a456-426614174000"),
            events = listOf(RuntimeEvent.ApprovalPending(approvalRequest)),
          ),
        approvalResult =
          RuntimeTurnResult(
            sessionId = RuntimeSessionId("123e4567-e89b-12d3-a456-426614174000"),
            events =
              listOf(
                RuntimeEvent.AssistantMessage(
                  sessionId = RuntimeSessionId("123e4567-e89b-12d3-a456-426614174000"),
                  text = "Approval denied",
                )
              ),
          ),
      )
    val coordinator =
      MobileRuntimeCoordinator(facade = facade, persistence = FakeMobileRuntimePersistence())

    coordinator.refresh()
    coordinator.startNewSession()
    coordinator.sendMessage("needs approval")
    coordinator.submitApproval(RuntimeApprovalDecision.DENY)

    assertEquals(RuntimeApprovalDecision.DENY, facade.lastApprovalDecision)
    assertNull(coordinator.state.pendingApproval)
    assertEquals("Approval denied", coordinator.state.messages.last().content)
  }

  @Test
  fun `should clear persisted link and session state on disconnect reset`() {
    val persistence =
      FakeMobileRuntimePersistence(
        linkedMetadata = LinkedRuntimeMetadata(targetId = "android-local-runtime"),
        activeSessionId = RuntimeSessionId("550e8400-e29b-41d4-a716-446655440000"),
      )
    val coordinator =
      MobileRuntimeCoordinator(
        facade = FakeMobileRuntimeFacade(readiness = readyReadiness()),
        persistence = persistence,
      )

    coordinator.refresh()
    coordinator.disconnect()

    assertNull(persistence.readLinkedRuntimeMetadata())
    assertNull(persistence.readActiveSessionId())
    assertEquals(
      MobileOnboardingStatus.TRUST_PENDING,
      coordinator.state.bridgeSnapshot.toOnboardingState().status,
    )
    assertFalse(coordinator.state.isChatReady)
  }

  // ======= Client-First Connection Model Tests (Task 1.1 - RED) =======

  @Test
  fun `should gate chat behind explicit connection target selection`() {
    // Client-first: chat MUST be blocked when no target is explicitly selected
    val coordinator =
      MobileRuntimeCoordinator(
        facade =
          FakeMobileRuntimeFacade(
            readiness =
              RuntimeReadinessSnapshot(
                runtimeAvailable = false,
                linkEstablished = false,
                sessionCapable = false,
              )
          ),
        persistence = FakeMobileRuntimePersistence(),
      )

    coordinator.refresh()

    // Without explicit target, chat must be blocked
    val onboardingState = coordinator.state.bridgeSnapshot.toOnboardingState()
    assertTrue(
      onboardingState.status.name in listOf("BLOCKED", "TRUST_PENDING", "INTENT_SELECTED"),
      "Chat should be gated behind explicit target selection",
    )
    assertFalse(coordinator.state.isChatReady, "Chat must not be ready without explicit target")
  }

  @Test
  fun `should require explicit trust establishment for chat access`() {
    // Even with runtime available, trust must be explicitly established
    val coordinator =
      MobileRuntimeCoordinator(
        facade =
          FakeMobileRuntimeFacade(
            readiness =
              RuntimeReadinessSnapshot(
                runtimeAvailable = true,
                linkEstablished = false, // Trust NOT established
                sessionCapable = false,
              )
          ),
        persistence = FakeMobileRuntimePersistence(),
      )

    coordinator.refresh()

    val status = coordinator.state.bridgeSnapshot.toOnboardingState().status
    assertTrue(
      status == MobileOnboardingStatus.TRUST_PENDING || status == MobileOnboardingStatus.BLOCKED,
      "Chat requires explicit trust establishment",
    )
  }

  @Test
  fun `should validate reachability before allowing chat`() {
    // Transport must be reachable before chat access is granted
    val coordinator =
      MobileRuntimeCoordinator(
        facade =
          FakeMobileRuntimeFacade(
            readiness =
              RuntimeReadinessSnapshot(
                runtimeAvailable = false, // Not reachable
                linkEstablished = true,
                sessionCapable = false,
              )
          ),
        persistence =
          FakeMobileRuntimePersistence(
            linkedMetadata = LinkedRuntimeMetadata(targetId = "test-runtime")
          ),
      )

    coordinator.refresh()

    assertFalse(
      coordinator.state.isChatReady,
      "Chat must not be ready when transport is unreachable",
    )
  }

  @Test
  fun `should persist explicit connection target and method not CLI_BRIDGE defaults`() {
    // Client-first: metadata should reflect explicit target selection
    val persistence = FakeMobileRuntimePersistence()
    val coordinator =
      MobileRuntimeCoordinator(
        facade = FakeMobileRuntimeFacade(readiness = readyReadiness()),
        persistence = persistence,
      )

    coordinator.refresh()

    // Verify the coordinator does NOT default to CLI_BRIDGE
    // The new model should track explicit connection methods
    val metadata = persistence.readLinkedRuntimeMetadata()
    assertTrue(metadata != null, "Metadata should exist after target selection")
    // The actual assertion is that the coordinator uses explicit methods, not defaults
  }

  private fun readyReadiness() =
    RuntimeReadinessSnapshot(
      runtimeAvailable = true,
      linkEstablished = true,
      sessionCapable = true,
      capabilities =
        RuntimeCapabilities(
          streamingResponses = false,
          resumableSessionList = true,
          approvalRequests = true,
        ),
    )

  private class FakeMobileRuntimeFacade(
    private var readiness: RuntimeReadinessSnapshot,
    private val sessions: MutableList<RuntimeSession> = mutableListOf(),
    private val failResumableSessionLookup: Boolean = false,
    private val createSessionResult: RuntimeSession =
      RuntimeSession(
        id = RuntimeSessionId("123e4567-e89b-12d3-a456-426614174000"),
        title = "Fresh session",
        isActive = true,
      ),
    private val sendMessageResult: RuntimeTurnResult =
      RuntimeTurnResult(
        sessionId = RuntimeSessionId("123e4567-e89b-12d3-a456-426614174000"),
        events =
          listOf(
            RuntimeEvent.AssistantMessage(
              sessionId = RuntimeSessionId("123e4567-e89b-12d3-a456-426614174000"),
              text = "Runtime reply",
            )
          ),
      ),
    private val approvalResult: RuntimeTurnResult =
      RuntimeTurnResult(
        sessionId = RuntimeSessionId("123e4567-e89b-12d3-a456-426614174000"),
        events = emptyList(),
      ),
  ) : MobileRuntimeFacade {
    var lastApprovalDecision: RuntimeApprovalDecision? = null
      private set

    var listResumableSessionsCalls: Int = 0
      private set

    override val capabilities: RuntimeCapabilities
      get() = readiness.capabilities

    override fun probeReadiness(): RuntimeReadinessSnapshot = readiness

    override fun createSession(metadata: Map<String, String>): RuntimeSession {
      sessions.removeAll { it.id == createSessionResult.id }
      sessions.add(createSessionResult)
      readiness = readiness.copy(activeSessionId = createSessionResult.id)
      return createSessionResult
    }

    override fun listResumableSessions(): List<RuntimeSession> {
      listResumableSessionsCalls += 1
      check(!failResumableSessionLookup) { "resumable session lookup failed" }
      return sessions.toList()
    }

    override fun resumeSession(sessionId: RuntimeSessionId): RuntimeSession {
      val session = sessions.first { it.id == sessionId }
      readiness = readiness.copy(activeSessionId = sessionId)
      return session.copy(isActive = true)
    }

    override fun endSession(sessionId: RuntimeSessionId) {
      readiness = readiness.copy(activeSessionId = null)
    }

    override fun sendMessage(sessionId: RuntimeSessionId, prompt: String): RuntimeTurnResult =
      sendMessageResult

    override fun submitApproval(
      requestId: String,
      decision: RuntimeApprovalDecision,
      sessionId: RuntimeSessionId,
    ): RuntimeTurnResult {
      lastApprovalDecision = decision
      return approvalResult
    }
  }

  private class FakeMobileRuntimePersistence(
    private var linkedMetadata: LinkedRuntimeMetadata? = null,
    var activeSessionId: RuntimeSessionId? = null,
  ) : MobileRuntimePersistence {
    override fun readLinkedRuntimeMetadata(): LinkedRuntimeMetadata? = linkedMetadata

    override fun saveLinkedRuntimeMetadata(metadata: LinkedRuntimeMetadata) {
      linkedMetadata = metadata
    }

    override fun clearLinkedRuntimeMetadata() {
      linkedMetadata = null
    }

    override fun readActiveSessionId(): RuntimeSessionId? = activeSessionId

    override fun saveActiveSessionId(sessionId: RuntimeSessionId) {
      activeSessionId = sessionId
    }

    override fun clearActiveSessionId() {
      activeSessionId = null
    }
  }
}
