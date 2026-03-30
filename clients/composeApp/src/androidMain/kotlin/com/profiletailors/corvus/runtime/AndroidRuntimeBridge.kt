package com.profiletailors.corvus.runtime

import java.io.File
import java.io.IOException
import java.util.concurrent.TimeUnit

data class AndroidRuntimeBridgeConfig(
  val executable: String = "corvus",
  val arguments: List<String> = listOf("agent", "-m"),
  val defaultTimeoutMs: Long = 30_000,
  val workingDirectory: String? = null,
)

class AndroidRuntimeBridge(
  private val config: AndroidRuntimeBridgeConfig = AndroidRuntimeBridgeConfig()
) : RuntimeFacade {
  override val capabilities: RuntimeCapabilities =
    RuntimeCapabilities(
      streamingResponses = false,
      resumableSessionList = true,
      approvalRequests = true,
    )

  override fun probeReadiness(): RuntimeReadinessSnapshot =
    runCommand("__corvus_probe__")
      ?.lineSequence()
      ?.map { it.trim() }
      ?.filter { it.contains('=') }
      ?.associate {
        val index = it.indexOf('=')
        it.substring(0, index).trim() to it.substring(index + 1).trim()
      }
      ?.let { values ->
        RuntimeReadinessSnapshot(
          runtimeAvailable = values.booleanValue("runtime_available"),
          linkEstablished =
            values.booleanValue("link_established") || values.booleanValue("bridge_linked"),
          sessionCapable = values.booleanValue("session_capable"),
          activeSessionId =
            values["session_id"]?.takeIf { it.isNotBlank() }?.let(::RuntimeSessionId),
          capabilities =
            RuntimeCapabilities(
              streamingResponses = values.booleanValue("cap_streaming"),
              resumableSessionList = values.booleanValue("cap_resumable_sessions"),
              approvalRequests = values.booleanValue("cap_approval_requests"),
            ),
        )
      }
      ?: RuntimeReadinessSnapshot(
        runtimeAvailable = false,
        linkEstablished = false,
        sessionCapable = false,
        capabilities = capabilities,
      )

  override fun createSession(metadata: Map<String, String>): RuntimeSession =
    sessionFrom(requireNotNull(runCommand("__corvus_create_session__")))

  override fun listResumableSessions(): List<RuntimeSession> =
    requireNotNull(runCommand("__corvus_list_sessions__"))
      .lineSequence()
      .map { it.trim() }
      .filter { it.startsWith("session=") }
      .map { line ->
        val parts = line.substringAfter("session=").split('|')
        RuntimeSession(
          id = RuntimeSessionId(parts[0]),
          title = parts.getOrNull(1)?.ifBlank { null },
          isActive = parts.getOrNull(2)?.toBooleanStrictOrNull() ?: false,
        )
      }
      .toList()

  override fun resumeSession(sessionId: RuntimeSessionId): RuntimeSession =
    sessionFrom(requireNotNull(runCommand("__corvus_resume_session__${sessionId.value}")))

  override fun endSession(sessionId: RuntimeSessionId) {
    runCommand("__corvus_end_session__${sessionId.value}")
  }

  override fun sendMessage(sessionId: RuntimeSessionId, prompt: String): RuntimeTurnResult =
    turnFrom(sessionId, runCommand("__corvus_send_message__${sessionId.value}\u001f$prompt"), "")

  override fun submitApproval(
    requestId: String,
    decision: RuntimeApprovalDecision,
    sessionId: RuntimeSessionId,
  ): RuntimeTurnResult =
    turnFrom(
      sessionId,
      runCommand(
        "__corvus_submit_approval__${sessionId.value}\u001f$requestId\u001f${decision.name}"
      ),
      "",
    )

  private fun runCommand(prompt: String): String? {
    val command = buildList {
      add(config.executable)
      addAll(config.arguments)
      add(prompt)
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
      } catch (_: IOException) {
        return null
      }

    val finished = process.waitFor(config.defaultTimeoutMs, TimeUnit.MILLISECONDS)
    if (!finished) {
      process.destroyForcibly()
      return null
    }

    if (process.exitValue() != 0) {
      return null
    }

    return process.inputStream.bufferedReader().use { it.readText() }.trim()
  }

  private fun sessionFrom(rawOutput: String): RuntimeSession {
    val values = rawOutput.toValues()
    return RuntimeSession(
      id = RuntimeSessionId(requireNotNull(values["session_id"])),
      title = values["session_title"]?.takeIf { it.isNotBlank() },
      isActive = values.booleanValue("session_active"),
    )
  }

  private fun turnFrom(
    sessionId: RuntimeSessionId,
    rawOutput: String?,
    fallback: String,
  ): RuntimeTurnResult {
    if (rawOutput == null) {
      return RuntimeTurnResult(
        sessionId = sessionId,
        events =
          listOf(
            RuntimeEvent.Failure(
              sessionId = sessionId,
              message = "Android runtime bridge unavailable.",
              recoverable = true,
            )
          ),
      )
    }

    val values = rawOutput.toValues()
    val message = values["assistant_message"]?.takeIf { it.isNotBlank() } ?: fallback
    return RuntimeTurnResult(
      sessionId = sessionId,
      events = listOf(RuntimeEvent.AssistantMessage(sessionId = sessionId, text = message)),
    )
  }

  private fun String.toValues(): Map<String, String> =
    lineSequence()
      .map { it.trim() }
      .filter { it.contains('=') }
      .associate {
        val index = it.indexOf('=')
        it.substring(0, index).trim() to it.substring(index + 1).trim()
      }

  private fun Map<String, String>.booleanValue(key: String): Boolean =
    when (this[key]?.lowercase()) {
      "1",
      "true",
      "yes",
      "ready" -> true
      else -> false
    }
}
