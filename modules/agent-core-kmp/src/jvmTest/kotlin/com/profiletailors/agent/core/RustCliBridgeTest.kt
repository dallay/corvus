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
}
