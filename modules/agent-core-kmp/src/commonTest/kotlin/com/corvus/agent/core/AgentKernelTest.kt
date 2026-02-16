package com.corvus.agent.core

import kotlin.test.Test
import kotlin.test.assertEquals

class AgentKernelTest {
  @Test
  fun `should expose bootstrap name`() {
    assertEquals("corvus-agent-core", AgentKernel.name)
  }
}
