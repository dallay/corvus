package com.profiletailors.corvus.ui.chat

import kotlin.test.Test
import kotlin.test.assertEquals

class ChatWorkspaceDefaultsTest {

  @Test
  fun `should create default workspace state`() {
    val state = ChatWorkspaceDefaults.state("MyAgent")

    assertEquals("MyAgent", state.modelName)
    assertEquals("Escribe un mensaje...", state.inputPlaceholder)
    assertEquals("Hola, soy MyAgent. En que puedo ayudarte?", state.welcomeMessage)
  }

  @Test
  fun `should use default agent name`() {
    val state = ChatWorkspaceDefaults.state()

    assertEquals(ChatWorkspaceDefaults.DefaultAgentName, state.modelName)
    assertEquals("Corvus Agent", ChatWorkspaceDefaults.DefaultAgentName)
  }

  @Test
  fun `should expose default gateway URL`() {
    assertEquals("http://127.0.0.1:3000", ChatWorkspaceDefaults.DefaultGatewayBaseUrl)
  }
}
