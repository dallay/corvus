package com.profiletailors.agent.core

import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertIs
import kotlin.test.assertTrue

class RustCliBridgeTest {
  @Test
  fun `should return success for zero exit command`() {
    val bridge =
      RustCliBridge(
        config =
          RustCliBridgeConfig(
            executable = "sh",
            arguments = listOf("-c", "printf '%s' \"$1\"", "bridge"),
          )
      )

    val result = bridge.invoke(CoreInvocation(prompt = "hello-bridge"))
    val success = assertIs<CoreResult.Success>(result)

    assertEquals("hello-bridge", success.output.text)
    assertEquals("rust-cli", success.output.transport)
  }

  @Test
  fun `should return failure for non zero exit`() {
    val bridge =
      RustCliBridge(
        config =
          RustCliBridgeConfig(
            executable = "sh",
            arguments = listOf("-c", "echo bridge-error >&2; exit 7", "bridge"),
          )
      )

    val result = bridge.invoke(CoreInvocation(prompt = "ignored"))
    val failure = assertIs<CoreResult.Failure>(result)

    assertTrue(failure.message.contains("code 7"))
    assertTrue(failure.details.orEmpty().contains("bridge-error"))
    assertTrue(failure.recoverable)
  }

  @Test
  fun `should return timeout failure`() {
    val bridge =
      RustCliBridge(
        config =
          RustCliBridgeConfig(
            executable = "sh",
            arguments = listOf("-c", "sleep 2", "bridge"),
            defaultTimeoutMs = 25,
          )
      )

    val result = bridge.invoke(CoreInvocation(prompt = "ignored"))
    val failure = assertIs<CoreResult.Failure>(result)

    assertTrue(failure.message.contains("timed out"))
    assertTrue(failure.recoverable)
  }

  @Test
  fun `should handle partial output before timeout without descriptor leak`() {
    val bridge =
      RustCliBridge(
        config =
          RustCliBridgeConfig(
            executable = "sh",
            arguments = listOf("-c", "echo 'partial output'; sleep 2", "bridge"),
            defaultTimeoutMs = 50,
          )
      )

    val result = bridge.invoke(CoreInvocation(prompt = "ignored"))
    val failure = assertIs<CoreResult.Failure>(result)

    assertTrue(failure.message.contains("timed out"))
    assertTrue(failure.recoverable)
  }

  @Test
  fun `should fail when executable is missing`() {
    val bridge =
      RustCliBridge(config = RustCliBridgeConfig(executable = "definitely-missing-corvus-binary"))

    val result = bridge.invoke(CoreInvocation(prompt = "ignored"))
    val failure = assertIs<CoreResult.Failure>(result)

    assertTrue(failure.message.contains("Unable to start Rust bridge executable"))
  }
}
