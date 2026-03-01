package com.profiletailors.corvus.ui.chat

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class ChatComponentsTest {

  @Test
  fun `should normalize endpoint URLs`() {
    assertEquals("http://api.corvus.ai/webhook", endpointUrl("http://api.corvus.ai", "/webhook"))
    assertEquals("http://api.corvus.ai/webhook", endpointUrl("http://api.corvus.ai/", "/webhook"))
    assertEquals(
      "http://api.corvus.ai/webhook",
      endpointUrl("  http://api.corvus.ai  ", "/webhook"),
    )
    assertEquals("/webhook", endpointUrl("", "/webhook"))
    assertEquals("/webhook", endpointUrl("  ", "/webhook"))
  }

  @Test
  fun `should build local assistant reply with gateway info`() {
    val gateway =
      AgentGatewayConfig(
        baseUrl = "http://localhost:3000",
        pairingCode = "123",
        bearerToken = "token-abc",
        webhookSecret = "secret",
      )

    val reply = buildLocalAssistantReply("hello", "Corvus", gateway)

    assertTrue(reply.contains("[Corvus]"), "Reply should contain model name")
    assertTrue(reply.contains("hello"), "Reply should contain original prompt")
    assertTrue(reply.contains("http://localhost:3000/webhook"), "Reply should contain webhook URL")
    assertTrue(reply.contains("con token"), "Reply should indicate token presence")
  }

  @Test
  fun `should build local assistant reply without token`() {
    val gateway =
      AgentGatewayConfig(
        baseUrl = "http://localhost:3000",
        pairingCode = "123",
        bearerToken = "",
        webhookSecret = "secret",
      )

    val reply = buildLocalAssistantReply("hello", "Corvus", gateway)

    assertTrue(reply.contains("sin token"), "Reply should indicate token absence")
  }
}
