package com.profiletailors.agent.core

import java.io.File
import java.io.IOException
import java.util.concurrent.TimeUnit

data class RustCliBridgeConfig(
  val executable: String = "corvus",
  val arguments: List<String> = listOf("agent", "-m"),
  val defaultTimeoutMs: Long = 30_000,
  val workingDirectory: String? = null,
)

class RustCliBridge(private val config: RustCliBridgeConfig = RustCliBridgeConfig()) :
  AgentCoreBridge {
  override fun invoke(invocation: CoreInvocation): CoreResult {
    val timeoutMs = invocation.timeoutMs ?: config.defaultTimeoutMs
    if (timeoutMs <= 0) {
      return CoreResult.Failure(
        message = "Timeout must be greater than zero milliseconds.",
        recoverable = false,
      )
    }

    val command = buildList {
      add(config.executable)
      addAll(config.arguments)
      add(invocation.prompt)
    }

    val process =
      try {
        ProcessBuilder(command)
          .apply {
            redirectErrorStream(true)
            if (!config.workingDirectory.isNullOrBlank()) {
              directory(File(config.workingDirectory))
            }
          }
          .start()
      } catch (error: IOException) {
        return CoreResult.Failure(
          message = "Unable to start Rust bridge executable '${config.executable}'.",
          details = error.message,
          recoverable = true,
        )
      }

    val finished = process.waitFor(timeoutMs, TimeUnit.MILLISECONDS)
    if (!finished) {
      process.inputStream.close()
      process.errorStream.close()
      process.destroyForcibly()
      return CoreResult.Failure(
        message = "Rust bridge timed out after ${timeoutMs}ms.",
        recoverable = true,
      )
    }

    val output = process.inputStream.bufferedReader().use { it.readText() }.trim()
    val exitCode = process.exitValue()

    if (exitCode != 0) {
      val details = output.ifBlank { "No output returned by Rust bridge." }
      return CoreResult.Failure(
        message = "Rust bridge exited with code ${exitCode}.",
        details = details,
        recoverable = true,
      )
    }

    return CoreResult.Success(
      output = CoreOutput(text = output, transport = "rust-cli", rawOutput = output)
    )
  }
}
