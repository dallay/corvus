package com.profiletailors.agent.core

import kotlin.test.Test
import kotlin.test.assertEquals

class AgentKernelTest {
  @Test
  fun `should expose module identity`() {
    assertEquals("corvus-agent-core", AgentKernel.name)
  }

  @Test
  fun `should expose contract version`() {
    assertEquals("0.1", AgentKernel.contractVersion)
  }
}
