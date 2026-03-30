package com.profiletailors.corvus.ui.chat

import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

/**
 * Tests for client-first onboarding copy. These tests verify that the UI copy uses client-first
 * language instead of host-first language.
 *
 * Per the delta specs:
 * - Desktop, Android, and iOS MUST guide users to connect to an existing runtime
 * - Copy MUST NOT refer to "local CLI bridge" or "local Corvus" as the default path
 * - Copy MUST describe connecting to an existing runtime, not spawning local processes
 */
class ClientFirstCopyTest {

  @Test
  fun `onboarding copy should NOT refer to local CLI bridge as default`() {
    val bridgeState =
      MobileBridgeUiState(
        platformName = "Desktop",
        snapshot =
          MobileBridgeSnapshot(
            runtimeAvailable = true,
            linkEstablished = false,
            sessionCapable = true,
          ),
      )

    val headline = bridgeStateHeadline(bridgeState)
    val description = bridgeStateDescription(bridgeState)

    // Client-first: should NOT mention local CLI bridge
    assertFalse(
      headline.contains("local", ignoreCase = true) &&
        headline.contains("bridge", ignoreCase = true),
      "Headline should not mention 'local bridge': $headline",
    )
    assertFalse(
      description.contains("CLI bridge", ignoreCase = true),
      "Description should not mention 'CLI bridge': $description",
    )
  }

  @Test
  fun `onboarding copy should describe connecting to existing runtime`() {
    val bridgeState =
      MobileBridgeUiState(
        platformName = "Android",
        snapshot =
          MobileBridgeSnapshot(
            runtimeAvailable = true,
            linkEstablished = false,
            sessionCapable = false,
          ),
      )

    val headline = bridgeStateHeadline(bridgeState)
    val description = bridgeStateDescription(bridgeState)

    // Client-first: should describe connecting to an existing runtime
    assertTrue(
      headline.contains("connect", ignoreCase = true) ||
        headline.contains("target", ignoreCase = true) ||
        headline.contains("endpoint", ignoreCase = true),
      "Headline should mention connect/target/endpoint: $headline",
    )
  }

  @Test
  fun `onboarding copy should NOT assume local corvus installation`() {
    val bridgeState =
      MobileBridgeUiState(
        platformName = "Android",
        snapshot =
          MobileBridgeSnapshot(
            runtimeAvailable = false,
            linkEstablished = false,
            sessionCapable = false,
          ),
      )

    val recovery = bridgeStateRecovery(bridgeState)

    // Client-first: should NOT instruct user to install or run local corvus
    assertFalse(
      recovery.contains("install", ignoreCase = true) &&
        recovery.contains("corvus", ignoreCase = true),
      "Recovery should not mention installing Corvus: $recovery",
    )
    assertFalse(
      recovery.contains("run", ignoreCase = true) && recovery.contains("corvus", ignoreCase = true),
      "Recovery should not mention running Corvus: $recovery",
    )
  }

  @Test
  fun `config panel should NOT default to local bridge transport`() {
    val bridgeState =
      MobileBridgeUiState(
        platformName = "Desktop",
        snapshot =
          MobileBridgeSnapshot(
            runtimeAvailable = true,
            linkEstablished = true,
            sessionCapable = true,
          ),
      )

    val safeDiagnostics = buildSafeDiagnosticLines(bridgeState, targetLabel = "my-runtime")

    // Client-first: transport should NOT be described as "local CLI bridge"
    assertFalse(
      safeDiagnostics.any {
        it.contains("local", ignoreCase = true) && it.contains("CLI", ignoreCase = true)
      },
      "Safe diagnostics should not mention local CLI: $safeDiagnostics",
    )
  }

  @Test
  fun `onboarding should expose endpoint configuration when no target configured`() {
    val bridgeState =
      MobileBridgeUiState(
        platformName = "Desktop",
        snapshot =
          MobileBridgeSnapshot(
            runtimeAvailable = false,
            linkEstablished = false,
            sessionCapable = false,
            recoveryOverride = MobileRecoveryKind.NO_TARGET_CONFIGURED,
          ),
      )

    val recovery = bridgeStateRecovery(bridgeState)

    // Client-first: should guide user to configure target/endpoint
    assertTrue(
      recovery.contains("endpoint", ignoreCase = true) ||
        recovery.contains("target", ignoreCase = true) ||
        recovery.contains("configure", ignoreCase = true),
      "Recovery should mention configuring target/endpoint: $recovery",
    )
  }

  @Test
  fun `platforms should show supported connection methods in diagnostics`() {
    val desktopState =
      MobileBridgeUiState(
        platformName = "Desktop",
        snapshot =
          MobileBridgeSnapshot(
            runtimeAvailable = true,
            linkEstablished = true,
            sessionCapable = true,
          ),
      )

    val androidState =
      MobileBridgeUiState(
        platformName = "Android",
        snapshot =
          MobileBridgeSnapshot(
            runtimeAvailable = true,
            linkEstablished = true,
            sessionCapable = true,
          ),
      )

    val desktopDiagnostics = buildSafeDiagnosticLines(desktopState, targetLabel = "desktop-runtime")
    val androidDiagnostics = buildSafeDiagnosticLines(androidState, targetLabel = "android-runtime")

    // Both should show target information
    assertTrue(
      desktopDiagnostics.any { it.contains("Target:") },
      "Desktop diagnostics should show target: $desktopDiagnostics",
    )
    assertTrue(
      androidDiagnostics.any { it.contains("Target:") },
      "Android diagnostics should show target: $androidDiagnostics",
    )
  }
}
