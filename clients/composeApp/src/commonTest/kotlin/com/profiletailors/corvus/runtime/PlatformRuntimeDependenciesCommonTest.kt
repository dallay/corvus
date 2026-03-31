package com.profiletailors.corvus.runtime

import com.profiletailors.corvus.ui.chat.MobileBridgeSnapshot
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertIs
import kotlin.test.assertSame
import kotlin.test.assertTrue
import kotlin.test.fail

class PlatformRuntimeDependenciesCommonTest {
  private val iosUnavailableMessage = iosCompanionUnavailableMessage()

  @Test
  fun `should document the missing iOS companion infrastructure explicitly`() {
    val message = iosCompanionUnavailableMessage()

    IOS_COMPANION_MISSING_INFRASTRUCTURE.forEach { missingPiece ->
      assertTrue(message.contains(missingPiece), "Expected message to mention: $missingPiece")
    }
    assertFalse(
      message.contains(
        "no Swift or Objective-C companion client has been installed into ComposeApp"
      ),
      "Concrete fallback client now exists; only transport blockers should remain in the message",
    )
  }

  @Test
  fun `should create runtime facade when preview mode is disabled`() {
    var invocationCount = 0
    val runtimeFacade =
      object :
        RuntimeFacade by FailClosedRuntimeFacade(unavailableReason = "runtime unavailable") {}

    val resolved =
      createPreviewAwareRuntimeFacade(initialBridgeSnapshot = null) {
        invocationCount += 1
        runtimeFacade
      }

    assertSame(runtimeFacade, resolved)
    assertEquals(1, invocationCount)
  }

  @Test
  fun `should keep preview facade when a bridge snapshot is injected`() {
    val resolved =
      createPreviewAwareRuntimeFacade(
        initialBridgeSnapshot =
          MobileBridgeSnapshot(
            runtimeAvailable = true,
            linkEstablished = true,
            sessionCapable = true,
          )
      ) {
        fail("runtime factory should not run in preview mode")
      }

    assertIs<PreviewMobileRuntimeFacade>(resolved)
  }

  @Test
  fun `should fail closed when runtime wiring is unavailable for the build`() {
    val facade =
      FailClosedRuntimeFacade(
        unavailableReason = iosUnavailableMessage,
        environmentSupported = false,
      )
    val sessionId = RuntimeSessionId("123e4567-e89b-12d3-a456-426614174000")

    val readiness = facade.probeReadiness()
    val sendResult = facade.sendMessage(sessionId = sessionId, prompt = "hello")
    val failure = sendResult.events.single() as? RuntimeEvent.Failure

    assertFalse(readiness.runtimeAvailable)
    assertFalse(readiness.environmentSupported)
    assertEquals(
      RuntimeRecoveryKind.ENVIRONMENT_UNSUPPORTED,
      readiness.toOnboardingState().recoveryKind,
    )
    assertEquals(iosUnavailableMessage, failure?.message)
    assertFalse(failure?.recoverable ?: true)
    assertTrue(facade.listResumableSessions().isEmpty())
  }
}
