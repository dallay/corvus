package com.profiletailors.agent.core

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertFalse
import kotlin.test.assertIs
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
}
