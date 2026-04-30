package com.profiletailors.corvus.ui.chat

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFalse
import kotlin.test.assertTrue

class SessionHistoryTest {
  @Test
  fun `should truncate long session IDs for display`() {
    val longId = "550e8400-e29b-41d4-a716-446655440000"
    val truncated = truncateSessionId(longId)
    assertEquals("550e8400…", truncated)
    assertTrue(truncated.length < longId.length)
  }

  @Test
  fun `should not truncate short session IDs`() {
    val shortId = "abc123"
    val result = truncateSessionId(shortId)
    assertEquals(shortId, result)
  }

  @Test
  fun `should expose only safe fields in session history — no memory keys or categories`() {
    val safeLines = buildSafeDiagnosticLines(
      bridgeState = MobileBridgeUiState(
        platformName = "Android",
        snapshot = MobileBridgeSnapshot(
          runtimeAvailable = true,
          linkEstablished = true,
          sessionCapable = true,
        ),
      ),
      targetLabel = "test-target",
    )

    assertFalse(safeLines.any { it.contains("memory", ignoreCase = true) })
    assertFalse(safeLines.any { it.contains("category", ignoreCase = true) })
    assertFalse(safeLines.any { it.contains("key", ignoreCase = true) })
    assertTrue(safeLines.any { it.contains("Target:") })
  }
}
