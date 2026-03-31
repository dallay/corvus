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
  // Default to zeroed capabilities - actual capabilities determined by probe
  override val capabilities: RuntimeCapabilities =
    RuntimeCapabilities(
      streamingResponses = false,
      resumableSessionList = false,
      approvalRequests = false,
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
        val hasValidCapabilities =
          values["cap_streaming"] != null ||
            values["cap_resumable_sessions"] != null ||
            values["cap_approval_requests"] != null
        val probedCapabilities =
          if (hasValidCapabilities) {
            RuntimeCapabilities(
              streamingResponses = values.booleanValue("cap_streaming"),
              resumableSessionList = values.booleanValue("cap_resumable_sessions"),
              approvalRequests = values.booleanValue("cap_approval_requests"),
            )
          } else {
            RuntimeCapabilities(
              streamingResponses = false,
              resumableSessionList = false,
              approvalRequests = false,
            )
          }
        RuntimeReadinessSnapshot(
          runtimeAvailable = values.booleanValue("runtime_available"),
          linkEstablished =
            values.booleanValue("link_established") || values.booleanValue("bridge_linked"),
          sessionCapable = values.booleanValue("session_capable"),
          activeSessionId =
            values["session_id"]?.takeIf { it.isNotBlank() }?.let(::RuntimeSessionId),
          capabilities = probedCapabilities,
        )
      }
      ?: RuntimeReadinessSnapshot(
        runtimeAvailable = false,
        linkEstablished = false,
        sessionCapable = false,
        capabilities =
          RuntimeCapabilities(
            streamingResponses = false,
            resumableSessionList = false,
            approvalRequests = false,
          ),
      )

  override fun createSession(metadata: Map<String, String>): RuntimeSession =
    sessionFrom(
      requireNotNull(runCommand("__corvus_create_session__")) {
        "runCommand('__corvus_create_session__') returned null in createSession(metadata=$metadata)"
      }
    )

  override fun listResumableSessions(): List<RuntimeSession> =
    requireNotNull(runCommand("__corvus_list_sessions__")) {
        "runCommand('__corvus_list_sessions__') returned null in listResumableSessions"
      }
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
    sessionFrom(
      requireNotNull(runCommand("__corvus_resume_session__${sessionId.value}")) {
        "runCommand('__corvus_resume_session__${sessionId.value}') returned null in resumeSession(sessionId=$sessionId)"
      }
    )

  override fun endSession(sessionId: RuntimeSessionId) {
    val result = runCommand("__corvus_end_session__${sessionId.value}")
    if (result == null) {
      error("Failed to end session ${sessionId.value}: runtime unavailable")
    }
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
    val process = startProcess(prompt) ?: return null
    val output = drainOutputWithTimeout(process) ?: return null
    return if (process.exitValue() == 0) output else null
  }

  private fun startProcess(prompt: String): Process? {
    val command = buildList {
      add(config.executable)
      addAll(config.arguments)
      add(prompt)
    }
    return try {
      ProcessBuilder(command)
        .apply {
          redirectErrorStream(true)
          if (!config.workingDirectory.isNullOrBlank()) {
            directory(File(config.workingDirectory))
          }
        }
        .start()
    } catch (_: IOException) {
      null
    }
  }

  private fun drainOutputWithTimeout(process: Process): String? {
    val outputBuilder = StringBuilder()
    val readerThread = Thread {
      try {
        process.inputStream.bufferedReader().use { reader ->
          var line: String?
          while (reader.readLine().also { line = it } != null) {
            outputBuilder.append(line).append("\n")
          }
        }
      } catch (e: Exception) {
        android.util.Log.d("AndroidRuntimeBridge", "Reader thread exception: ${e.message}", e)
      }
    }
    readerThread.start()

    val finished = process.waitFor(config.defaultTimeoutMs, TimeUnit.MILLISECONDS)
    if (!finished) {
      process.destroyForcibly()
      readerThread.interrupt()
      return null
    }

    readerThread.join(1000)
    return outputBuilder.toString().trim()
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
    val event =
      when {
        rawOutput == null ->
          RuntimeEvent.Failure(
            sessionId = sessionId,
            message = "Android runtime bridge unavailable.",
            recoverable = true,
          )
        else -> {
          val values = rawOutput.toValues()
          val approvalRequestId = values["approval_request_id"]
          val approvalMessage = values["approval_message"]
          if (approvalRequestId != null && approvalMessage != null) {
            RuntimeEvent.ApprovalPending(
              request =
                RuntimeApprovalRequest(
                  id = approvalRequestId,
                  sessionId = sessionId,
                  toolLabel = "Tool",
                  reason = approvalMessage,
                )
            )
          } else {
            val message = values["assistant_message"]?.takeIf { it.isNotBlank() } ?: fallback
            RuntimeEvent.AssistantMessage(sessionId = sessionId, text = message)
          }
        }
      }
    return RuntimeTurnResult(sessionId = sessionId, events = listOf(event))
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
