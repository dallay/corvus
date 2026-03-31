package com.profiletailors.agent.core

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertTrue

class RustCliBridgeTest {
  @Test
  fun `should return success for zero exit command`() {
    val bridge =
      RustCliBridge(
        config =
          RustCliBridgeConfig(
            executable = "sh",
            arguments = listOf("-c", "printf '%s' \"$1\"", "bridge"),
          )
      )

    val result = bridge.invoke(CoreInvocation(prompt = "hello-bridge"))
    val success = assertIs<CoreResult.Success>(result)

    assertEquals("hello-bridge", success.output.text)
    assertEquals("rust-cli", success.output.transport)
  }

  @Test
  fun `should return failure for non zero exit`() {
    val bridge =
      RustCliBridge(
        config =
          RustCliBridgeConfig(
            executable = "sh",
            arguments = listOf("-c", "echo bridge-error >&2; exit 7", "bridge"),
          )
      )

    val result = bridge.invoke(CoreInvocation(prompt = "ignored"))
    val failure = assertIs<CoreResult.Failure>(result)

    assertTrue(failure.message.contains("code 7"))
    assertTrue(failure.details.orEmpty().contains("bridge-error"))
    assertTrue(failure.recoverable)
  }

  @Test
  fun `should return timeout failure`() {
    val bridge =
      RustCliBridge(
        config =
          RustCliBridgeConfig(
            executable = "sh",
            arguments = listOf("-c", "sleep 2", "bridge"),
            defaultTimeoutMs = 25,
          )
      )

    val result = bridge.invoke(CoreInvocation(prompt = "ignored"))
    val failure = assertIs<CoreResult.Failure>(result)

    assertTrue(failure.message.contains("timed out"))
    assertTrue(failure.recoverable)
  }

  @Test
  fun `should handle partial output before timeout without descriptor leak`() {
    val bridge =
      RustCliBridge(
        config =
          RustCliBridgeConfig(
            executable = "sh",
            arguments = listOf("-c", "echo 'partial output'; sleep 2", "bridge"),
            defaultTimeoutMs = 50,
          )
      )

    for (i in 1..50) {
      val result = bridge.invoke(CoreInvocation(prompt = "ignored"))
      val failure = assertIs<CoreResult.Failure>(result)

      assertTrue(failure.message.contains("timed out"))
      assertTrue(failure.recoverable)
    }
  }

  @Test
  fun `should fail when executable is missing`() {
    val bridge =
      RustCliBridge(config = RustCliBridgeConfig(executable = "definitely-missing-corvus-binary"))

    val result = bridge.invoke(CoreInvocation(prompt = "ignored"))
    val failure = assertIs<CoreResult.Failure>(result)

    assertTrue(failure.message.contains("Unable to start Rust bridge executable"))
  }

  @Test
  fun `should classify discovered bridge as trust pending until linked`() {
    val snapshot =
      RustCliBridge.parseBridgeProbe(
        """
        runtime_available=true
        link_established=false
        session_capable=true
        """
          .trimIndent()
      )

    val state = snapshot.toOnboardingState()

    assertTrue(snapshot.runtimeAvailable)
    assertFalse(snapshot.linkEstablished)
    assertEquals(OnboardingState.Status.TRUST_PENDING, state.state)
    assertEquals(TrustMode.BRIDGE_LINKED, state.trustMode)
    assertEquals(TransportMode.CLI_BRIDGE, state.transportMode)
    assertEquals("trust_pending", state.stateLabel)
  }

  @Test
  fun `should classify linked bridge as session pending before session creation`() {
    val snapshot =
      RustCliBridge.parseBridgeProbe(
        """
        runtime_available=true
        link_established=true
        session_capable=true
        """
          .trimIndent()
      )

    val state = snapshot.toOnboardingState()

    assertEquals(OnboardingState.Status.SESSION_PENDING, state.state)
    assertTrue(state.canRetry)
    assertFalse(state.canResume)
  }

  @Test
  fun `should classify linked bridge without session capability as blocked recovery`() {
    val snapshot =
      RustCliBridge.parseBridgeProbe(
        """
        runtime_available=true
        link_established=true
        session_capable=false
        """
          .trimIndent()
      )

    val state = snapshot.toOnboardingState()

    assertEquals(OnboardingState.Status.BLOCKED, state.state)
    assertEquals(RecoveryKind.LINKED_BUT_NOT_SESSION_READY, state.recoveryKind)
    assertTrue(state.canRetry)
    assertEquals("linked_but_not_session_ready", state.recoveryLabel)
  }

  @Test
  fun `should expose canonical onboarding transition labels for bridge observability`() {
    assertEquals(
      "trust_pending__to__session_pending",
      onboardingTransitionLabel(
        from = OnboardingState.Status.TRUST_PENDING,
        to = OnboardingState.Status.SESSION_PENDING,
      ),
    )
  }

  @Test
  fun `should classify unsupported bridge environments before linking`() {
    val snapshot =
      RustCliBridge.parseBridgeProbe(
        rawOutput = "runtime_available=true\nlink_established=false\nsession_capable=false",
        environmentSupported = false,
      )

    val state = snapshot.toOnboardingState()

    assertEquals(OnboardingState.Status.BLOCKED, state.state)
    assertEquals(RecoveryKind.ENVIRONMENT_UNSUPPORTED, state.recoveryKind)
    assertFalse(state.canRetry)
  }

  @Test
  fun `should classify linked bridge with resumable session as session ready`() {
    val snapshot =
      RustCliBridge.parseBridgeProbe(
        """
        runtime_available=true
        link_established=true
        session_capable=true
        session_id=550e8400-e29b-41d4-a716-446655440000
        """
          .trimIndent()
      )

    val state = snapshot.toOnboardingState()

    assertEquals(OnboardingState.Status.SESSION_READY, state.state)
    assertEquals("550e8400-e29b-41d4-a716-446655440000", snapshot.sessionId)
    assertTrue(state.canResume)
  }

  @Test
  fun `should expose readiness and shared capabilities through mobile facade`() {
    val bridge = scriptedBridge()

    val snapshot = bridge.probeReadiness()

    assertTrue(snapshot.runtimeAvailable)
    assertTrue(snapshot.linkEstablished)
    assertTrue(snapshot.sessionCapable)
    assertEquals(SessionId("550e8400-e29b-41d4-a716-446655440000"), snapshot.activeSessionId)
    assertFalse(snapshot.capabilities.streamingResponses)
    assertTrue(snapshot.capabilities.resumableSessionList)
    assertTrue(snapshot.capabilities.approvalRequests)
  }

  @Test
  fun `should create resume list and end sessions through mobile facade`() {
    val bridge = scriptedBridge()

    val created = bridge.createSession()
    val listed = bridge.listResumableSessions()
    val resumed = bridge.resumeSession(SessionId("550e8400-e29b-41d4-a716-446655440000"))
    bridge.endSession(SessionId("550e8400-e29b-41d4-a716-446655440000"))

    assertEquals(SessionId("123e4567-e89b-12d3-a456-426614174000"), created.id)
    assertEquals("Fresh mobile session", created.title)
    assertEquals(2, listed.size)
    assertEquals(SessionId("550e8400-e29b-41d4-a716-446655440000"), resumed.id)
    assertTrue(resumed.isActive)
  }

  @Test
  fun `should return synchronous assistant replies when streaming is unavailable`() {
    val bridge = scriptedBridge()

    val result =
      bridge.sendMessage(
        sessionId = SessionId("550e8400-e29b-41d4-a716-446655440000"),
        prompt = "hello runtime",
      )

    assertEquals(SessionId("550e8400-e29b-41d4-a716-446655440000"), result.sessionId)
    val message = assertIs<MobileRuntimeEvent.AssistantMessage>(result.events.single())
    assertEquals("Runtime reply from CLI bridge", message.text)
  }

  @Test
  fun `should map bridge timeouts to recoverable turn failures`() {
    val bridge =
      RustCliBridge(
        config =
          RustCliBridgeConfig(
            executable = "sh",
            arguments = listOf("-c", "sleep 2", "bridge"),
            defaultTimeoutMs = 25,
          )
      )

    val result =
      bridge.sendMessage(
        sessionId = SessionId("550e8400-e29b-41d4-a716-446655440000"),
        prompt = "hello runtime",
      )

    val failure = assertIs<MobileRuntimeEvent.Failure>(result.events.single())
    assertTrue(failure.recoverable)
    assertTrue(failure.message.contains("timed out"))
  }

  @Test
  fun `should round trip approval decisions through mobile facade`() {
    val bridge = scriptedBridge()

    val result =
      bridge.submitApproval(
        requestId = "approval-1",
        decision = MobileApprovalDecision.APPROVE,
        sessionId = SessionId("550e8400-e29b-41d4-a716-446655440000"),
      )

    val message = assertIs<MobileRuntimeEvent.AssistantMessage>(result.events.single())
    assertEquals("Approval recorded: APPROVE", message.text)
  }

  private fun scriptedBridge(): RustCliBridge =
    RustCliBridge(
      config =
        RustCliBridgeConfig(
          executable = "sh",
          arguments = listOf("-c", SCRIPTED_RUNTIME_COMMAND, "bridge"),
        )
    )

  private companion object {
    private const val SCRIPTED_RUNTIME_COMMAND =
      """
      case "$1" in
        __corvus_probe__)
          printf 'runtime_available=true\nlink_established=true\nsession_capable=true\nsession_id=550e8400-e29b-41d4-a716-446655440000\ncap_streaming=false\ncap_resumable_sessions=true\ncap_approval_requests=true'
          ;;
        __corvus_create_session__)
          printf 'session_id=123e4567-e89b-12d3-a456-426614174000\nsession_title=Fresh mobile session\nsession_active=true'
          ;;
        __corvus_list_sessions__)
          printf 'session=550e8400-e29b-41d4-a716-446655440000|Existing session|true\nsession=123e4567-e89b-12d3-a456-426614174000|Fresh mobile session|false'
          ;;
        __corvus_resume_session__*)
          printf 'session_id=550e8400-e29b-41d4-a716-446655440000\nsession_title=Existing session\nsession_active=true'
          ;;
        __corvus_end_session__*)
          printf 'ended=true'
          ;;
        __corvus_send_message__*)
          printf 'assistant_message=Runtime reply from CLI bridge'
          ;;
        __corvus_submit_approval__*)
          printf 'assistant_message=Approval recorded: APPROVE'
          ;;
        *)
          printf 'unknown command: %s' "$1" >&2
          exit 1
          ;;
      esac
      """
  }
}
