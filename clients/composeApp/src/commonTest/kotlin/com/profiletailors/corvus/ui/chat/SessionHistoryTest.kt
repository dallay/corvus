package com.profiletailors.corvus.ui.chat

import com.profiletailors.corvus.runtime.RuntimeSession
import com.profiletailors.corvus.runtime.RuntimeSessionId
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
  fun `should display session title when present instead of truncated ID`() {
    val sessionWithTitle =
      RuntimeSession(
        id = RuntimeSessionId("550e8400-e29b-41d4-a716-446655440000"),
        title = "My named session",
        isActive = false,
      )
    val displayText = sessionWithTitle.title ?: truncateSessionId(sessionWithTitle.id.value)
    assertEquals("My named session", displayText)
  }

  @Test
  fun `should fall back to truncated ID when session has no title`() {
    val sessionWithoutTitle =
      RuntimeSession(
        id = RuntimeSessionId("550e8400-e29b-41d4-a716-446655440000"),
        title = null,
        isActive = false,
      )
    val displayText = sessionWithoutTitle.title ?: truncateSessionId(sessionWithoutTitle.id.value)
    assertEquals("550e8400…", displayText)
    assertFalse(displayText.contains("memory", ignoreCase = true))
    assertFalse(displayText.contains("category", ignoreCase = true))
  }

  @Test
  fun `should not expose memory content in session display fields`() {
    val sessions =
      listOf(
        RuntimeSession(
          id = RuntimeSessionId("550e8400-e29b-41d4-a716-446655440000"),
          title = "Session A",
          isActive = true,
        ),
        RuntimeSession(
          id = RuntimeSessionId("123e4567-e89b-12d3-a456-426614174000"),
          title = null,
          isActive = false,
        ),
      )
    sessions.forEach { session ->
      val displayText = session.title ?: truncateSessionId(session.id.value)
      assertFalse(displayText.contains("memory", ignoreCase = true))
      assertFalse(displayText.contains("token", ignoreCase = true))
      assertFalse(displayText.contains("key", ignoreCase = true))
    }
  }
}
