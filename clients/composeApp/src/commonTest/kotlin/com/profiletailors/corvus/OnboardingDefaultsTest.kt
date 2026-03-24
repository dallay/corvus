package com.profiletailors.corvus

import com.profiletailors.corvus.ui.chat.MobileBridgeSnapshot
import com.profiletailors.corvus.ui.chat.MobileBridgeUiState
import com.profiletailors.corvus.ui.chat.MobileOnboardingStatus
import com.profiletailors.corvus.ui.chat.MobileRecoveryKind
import com.profiletailors.corvus.ui.chat.MobileTransportMode
import com.profiletailors.corvus.ui.chat.MobileTrustMode
import com.profiletailors.corvus.ui.chat.bridgeStateDescription
import com.profiletailors.corvus.ui.chat.bridgeStateHeadline
import com.profiletailors.corvus.ui.chat.bridgeStateRecovery
import com.profiletailors.corvus.ui.chat.mobileOnboardingRecoveryLabel
import com.profiletailors.corvus.ui.chat.mobileOnboardingStateLabel
import com.profiletailors.corvus.ui.chat.mobileOnboardingTransitionLabel
import com.profiletailors.corvus.ui.chat.onboardingStateLabel
import com.profiletailors.corvus.ui.onboarding.OnboardingDefaults
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class OnboardingDefaultsTest {

  @Test
  fun `should expose stable mobile onboarding steps`() {
    val steps = OnboardingDefaults.steps

    assertEquals(4, steps.size)
    assertEquals(4, steps.map { it.titleRes }.distinct().size)
    assertEquals(4, steps.map { it.descriptionRes }.distinct().size)
    assertEquals(
      listOf(
        MobileOnboardingStatus.RUNTIME_PATH_CONFIRMED,
        MobileOnboardingStatus.TRUST_PENDING,
        MobileOnboardingStatus.TRANSPORT_CONNECTING,
        MobileOnboardingStatus.SESSION_PENDING,
      ),
      steps.map { it.status },
    )
    assertTrue(steps.all { it.trustMode == MobileTrustMode.BRIDGE_LINKED })
    assertTrue(steps.all { it.transportMode == MobileTransportMode.CLI_BRIDGE })
  }

  @Test
  fun `should map mobile bridge defaults by platform capability`() {
    val androidState = defaultBridgeSnapshotFor(FakePlatform(name = "Android 15", isMobile = true))
    val iosState =
      defaultBridgeSnapshotFor(
        FakePlatform(
          name = "iOS 17",
          isMobile = true,
          bridgeAvailability = BridgeAvailability.COMPANION_REQUIRED,
        )
      )

    assertEquals(MobileOnboardingStatus.TRUST_PENDING, androidState.toOnboardingState().status)
    assertEquals(MobileOnboardingStatus.BLOCKED, iosState.toOnboardingState().status)
  }

  @Test
  fun `should expose canonical mobile observability labels`() {
    assertEquals(
      "trust_pending__to__session_pending",
      mobileOnboardingTransitionLabel(
        from = MobileOnboardingStatus.TRUST_PENDING,
        to = MobileOnboardingStatus.SESSION_PENDING,
      ),
    )
    assertEquals(
      "linked_but_not_session_ready",
      mobileOnboardingRecoveryLabel(MobileRecoveryKind.LINKED_BUT_NOT_SESSION_READY),
    )
  }

  @Test
  fun `should keep mobile trust copy on linking instead of HTTP pairing`() {
    val bridgeState =
      MobileBridgeUiState(
        platformName = "Android 15",
        snapshot =
          MobileBridgeSnapshot(
            runtimeAvailable = true,
            linkEstablished = false,
            sessionCapable = true,
          ),
      )

    val headline = bridgeStateHeadline(bridgeState)
    val description = bridgeStateDescription(bridgeState)

    assertEquals("Trust this surface", onboardingStateLabel(bridgeState.onboardingState))
    assertTrue(headline.contains("Link", ignoreCase = true))
    assertTrue(description.contains("linking", ignoreCase = true))
    assertFalse(description.contains("pairing code", ignoreCase = true))
    assertFalse(description.contains("bearer token", ignoreCase = true))
  }

  @Test
  fun `should keep mobile recovery guidance on the approved bridge path`() {
    val blockedState =
      MobileBridgeUiState(
        platformName = "iOS 17",
        snapshot =
          MobileBridgeSnapshot(
            runtimeAvailable = true,
            linkEstablished = false,
            sessionCapable = false,
            environmentSupported = false,
          ),
      )

    val recovery = bridgeStateRecovery(blockedState)

    assertTrue(recovery.contains("approved companion path", ignoreCase = true))
    assertFalse(recovery.contains("pairing code", ignoreCase = true))
    assertFalse(recovery.contains("bearer token", ignoreCase = true))
  }

  @Test
  fun `should expose normalized recovery labels comparable to web chat`() {
    val runtimeBlocked =
      MobileBridgeUiState(
        platformName = "Android 15",
        snapshot =
          MobileBridgeSnapshot(
            runtimeAvailable = false,
            linkEstablished = false,
            sessionCapable = false,
          ),
      )
    val sessionBlocked =
      MobileBridgeUiState(
        platformName = "Android 15",
        snapshot =
          MobileBridgeSnapshot(
            runtimeAvailable = true,
            linkEstablished = true,
            sessionCapable = false,
          ),
      )

    assertEquals("runtime_unavailable", runtimeBlocked.onboardingRecoveryLabel)
    assertEquals("linked_but_not_session_ready", sessionBlocked.onboardingRecoveryLabel)
  }

  @Test
  fun `should keep mobile chat intent explicit through session-first onboarding states`() {
    val pendingSession =
      MobileBridgeUiState(
        platformName = "Android 15",
        snapshot =
          MobileBridgeSnapshot(
            runtimeAvailable = true,
            linkEstablished = true,
            sessionCapable = true,
          ),
      )

    assertEquals(MobileOnboardingStatus.SESSION_PENDING, pendingSession.onboardingState.status)
    assertEquals(MobileTrustMode.BRIDGE_LINKED, pendingSession.onboardingState.trustMode)
    assertEquals(MobileTransportMode.CLI_BRIDGE, pendingSession.onboardingState.transportMode)
    assertFalse(pendingSession.isChatReady)
  }

  @Test
  fun `should keep mobile outcomes aligned with the broader shared onboarding matrix`() {
    val scenarios =
      listOf(
        Triple(
          MobileBridgeSnapshot(
            runtimeAvailable = true,
            linkEstablished = false,
            sessionCapable = true,
          ),
          "trust_pending",
          null,
        ),
        Triple(
          MobileBridgeSnapshot(
            runtimeAvailable = true,
            linkEstablished = true,
            sessionCapable = true,
          ),
          "session_pending",
          null,
        ),
        Triple(
          MobileBridgeSnapshot(
            runtimeAvailable = true,
            linkEstablished = true,
            sessionCapable = true,
            sessionId = "550e8400-e29b-41d4-a716-446655440000",
          ),
          "session_ready",
          null,
        ),
        Triple(
          MobileBridgeSnapshot(
            runtimeAvailable = false,
            linkEstablished = false,
            sessionCapable = false,
          ),
          "blocked",
          "runtime_unavailable",
        ),
      )

    scenarios.forEach { (mobileSnapshot, expectedStateLabel, expectedRecoveryLabel) ->
      val mobileState = mobileSnapshot.toOnboardingState()

      assertEquals(expectedStateLabel, mobileOnboardingStateLabel(mobileState.status))
      assertEquals(
        expectedRecoveryLabel,
        mobileState.recoveryKind?.let(::mobileOnboardingRecoveryLabel),
      )
      assertEquals(MobileTrustMode.BRIDGE_LINKED, mobileState.trustMode)
      assertEquals(MobileTransportMode.CLI_BRIDGE, mobileState.transportMode)
    }
  }

  @Test
  fun `should validate ready and session outcomes through the approved cli bridge transport`() {
    val pendingSession =
      MobileBridgeUiState(
        platformName = "Android 15",
        snapshot =
          MobileBridgeSnapshot(
            runtimeAvailable = true,
            linkEstablished = true,
            sessionCapable = true,
          ),
      )
    val readySession =
      MobileBridgeUiState(
        platformName = "Android 15",
        snapshot =
          MobileBridgeSnapshot(
            runtimeAvailable = true,
            linkEstablished = true,
            sessionCapable = true,
            sessionId = "550e8400-e29b-41d4-a716-446655440000",
          ),
      )

    assertEquals(MobileTransportMode.CLI_BRIDGE, pendingSession.onboardingState.transportMode)
    assertEquals(MobileOnboardingStatus.SESSION_PENDING, pendingSession.onboardingState.status)
    assertTrue(bridgeStateDescription(pendingSession).contains("bridge", ignoreCase = true))
    assertFalse(bridgeStateDescription(pendingSession).contains("gateway", ignoreCase = true))
    assertFalse(bridgeStateDescription(pendingSession).contains("pairing", ignoreCase = true))

    assertEquals(MobileTransportMode.CLI_BRIDGE, readySession.onboardingState.transportMode)
    assertEquals(MobileOnboardingStatus.SESSION_READY, readySession.onboardingState.status)
    assertTrue(readySession.isChatReady)
  }

  private data class FakePlatform(
    override val name: String,
    override val isMobile: Boolean,
    override val bridgeAvailability: BridgeAvailability = BridgeAvailability.LOCAL_BRIDGE,
  ) : Platform
}
