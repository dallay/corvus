package com.profiletailors.agent.core

import kotlin.test.Test
import kotlin.test.assertEquals
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
}
