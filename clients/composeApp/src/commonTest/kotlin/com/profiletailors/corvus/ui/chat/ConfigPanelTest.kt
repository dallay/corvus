package com.profiletailors.corvus.ui.chat

import kotlin.test.Test
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class ConfigPanelTest {
  @Test
  fun `should expose only parity-critical safe diagnostics`() {
    val lines =
      buildSafeDiagnosticLines(
        bridgeState =
          MobileBridgeUiState(
            platformName = "Android 15",
            snapshot =
              MobileBridgeSnapshot(
                runtimeAvailable = true,
                linkEstablished = true,
                sessionCapable = true,
              ),
          ),
        targetLabel = "android-local-runtime",
      )

    assertTrue(lines.any { it.contains("Target:") })
    assertTrue(lines.any { it.contains("Transport:") })
    assertTrue(lines.any { it.contains("timeout", ignoreCase = true) })
    assertFalse(lines.any { it.contains("pairing code", ignoreCase = true) })
    assertFalse(lines.any { it.contains("bearer token", ignoreCase = true) })
    assertFalse(lines.any { it.contains("webhook", ignoreCase = true) })
    assertFalse(lines.any { it.contains("gateway url", ignoreCase = true) })
    assertFalse(lines.any { it.contains("admin", ignoreCase = true) })
    assertFalse(lines.any { it.contains("provider", ignoreCase = true) })
    assertFalse(lines.any { it.contains("memory", ignoreCase = true) })
  }

  @Test
  fun `should describe reset options without unsafe controls`() {
    val lines = buildResetOptionLines()

    assertTrue(lines.any { it.contains("Retry", ignoreCase = true) })
    // Client-first: Changed from "Relink" to "Reconfigure"
    assertTrue(lines.any { it.contains("Reconfigure", ignoreCase = true) })
    assertTrue(lines.any { it.contains("Disconnect", ignoreCase = true) })
    assertFalse(lines.any { it.contains("pairing code", ignoreCase = true) })
    assertFalse(lines.any { it.contains("bearer token", ignoreCase = true) })
    assertFalse(lines.any { it.contains("gateway", ignoreCase = true) })
    assertFalse(lines.any { it.contains("admin", ignoreCase = true) })
  }
}
