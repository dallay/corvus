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
  fun `should fail when executable is missing`() {
    val bridge =
      RustCliBridge(config = RustCliBridgeConfig(executable = "definitely-missing-corvus-binary"))

    val result = bridge.invoke(CoreInvocation(prompt = "ignored"))
    val failure = assertIs<CoreResult.Failure>(result)

    assertTrue(failure.message.contains("Unable to start Rust bridge executable"))
  }

  @Test
  fun `should recover and succeed after a timeout failure`() {
    val bridge =
      RustCliBridge(
        config =
          RustCliBridgeConfig(
            executable = "sh",
            arguments =
              listOf(
                "-c",
                "if [ \"$1\" = \"slow\" ]; then sleep 1; else echo \"fast-success\"; fi",
                "bridge",
              ),
          )
      )

    // First call: timeout
    val timeoutResult = bridge.invoke(CoreInvocation(prompt = "slow", timeoutMs = 50))
    assertIs<CoreResult.Failure>(timeoutResult)
    assertTrue(timeoutResult.message.contains("timed out"))

    // Second call: success
    val successResult = bridge.invoke(CoreInvocation(prompt = "fast"))
    val success = assertIs<CoreResult.Success>(successResult)
    assertEquals("fast-success", success.output.text)
  }

  @Test
  fun `should handle large output without crashing`() {
    // Generate ~10KB of output.
    val bridge =
      RustCliBridge(
        config =
          RustCliBridgeConfig(
            executable = "python3",
            arguments = listOf("-c", "print('a' * 10240)"),
          )
      )

    val result = bridge.invoke(CoreInvocation(prompt = "ignored"))
    val success = assertIs<CoreResult.Success>(result)
    assertEquals(10240, success.output.text.length)
    assertTrue(success.output.text.all { it == 'a' })
  }
}
