package com.profiletailors.agent.core

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs

class AgentKernelTest {
  @Test
  fun `should expose module identity`() {
    assertEquals("corvus-agent-core", AgentKernel.name)
  }

  @Test
  fun `should expose contract version`() {
    assertEquals("0.1", AgentKernel.contractVersion)
  }

  @Test
  fun `should preserve legacy bridge invocation contract alongside mobile facade expansion`() {
    val bridge = AgentCoreBridge { invocation ->
      CoreResult.Success(CoreOutput(text = invocation.prompt, transport = "test-bridge"))
    }

    val result = bridge.invoke(CoreInvocation(prompt = "hello"))
    val success = assertIs<CoreResult.Success>(result)

    assertEquals("hello", success.output.text)
    assertEquals("test-bridge", success.output.transport)
  }
}
