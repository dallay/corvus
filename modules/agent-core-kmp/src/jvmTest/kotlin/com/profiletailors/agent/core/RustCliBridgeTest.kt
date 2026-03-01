package com.profiletailors.agent.core

import java.io.File
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
            arguments = listOf("-c", "echo bridge-error >&2; /usr/bin/env sh -c 'exit 7'", "bridge"),
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
  fun `should fail for invalid timeout`() {
    val bridge = RustCliBridge()
    val result = bridge.invoke(CoreInvocation(prompt = "hello", timeoutMs = 0))
    val failure = assertIs<CoreResult.Failure>(result)

    assertTrue(failure.message.contains("Timeout must be greater than zero"))
  }

  @Test
  fun `should honor working directory`() {
    val tempDir = System.getProperty("java.io.tmpdir")
    val bridge =
      RustCliBridge(
        config =
          RustCliBridgeConfig(
            executable = "sh",
            arguments = listOf("-c", "pwd", "bridge"),
            workingDirectory = tempDir,
          )
      )

    val result = bridge.invoke(CoreInvocation(prompt = "ignored"))
    val success = assertIs<CoreResult.Success>(result)

    // Normalize paths for comparison
    val expected = File(tempDir).absolutePath.removeSuffix("/")
    val actual = File(success.output.text).absolutePath.removeSuffix("/")
    assertEquals(expected, actual)
  }

  @Test
  fun `should return default details when output is blank on failure`() {
    val bridge =
      RustCliBridge(
        config =
          RustCliBridgeConfig(
            executable = "sh",
            arguments = listOf("-c", "/usr/bin/env sh -c 'exit 9'", "bridge"),
          )
      )

    val result = bridge.invoke(CoreInvocation(prompt = "ignored"))
    val failure = assertIs<CoreResult.Failure>(result)

    assertEquals("No output returned by Rust bridge.", failure.details)
  }
}
