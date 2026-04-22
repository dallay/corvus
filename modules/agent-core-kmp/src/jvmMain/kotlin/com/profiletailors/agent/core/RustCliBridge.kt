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

@Suppress(
  "TooManyFunctions"
) // Implements full AgentCoreBridge + MobileRuntimeFacade; split would be artificial
class RustCliBridge(private val config: RustCliBridgeConfig = RustCliBridgeConfig()) :
  AgentCoreBridge, MobileRuntimeFacade {
  override val capabilities: MobileRuntimeCapabilities =
    MobileRuntimeCapabilities(
      streamingResponses = false,
      resumableSessionList = true,
      approvalRequests = true,
    )

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
      runCatching { process.inputStream.close() }
      runCatching { process.errorStream.close() }
      runCatching { process.outputStream.close() }
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

  override fun probeReadiness(): MobileRuntimeReadinessSnapshot {
    val result = invoke(CoreInvocation(prompt = MOBILE_PROBE_PROMPT))
    return when (result) {
      is CoreResult.Success -> parseReadiness(result.output.text)
      is CoreResult.Failure ->
        MobileRuntimeReadinessSnapshot(
          runtimeAvailable = false,
          linkEstablished = false,
          sessionCapable = false,
          capabilities = capabilities,
        )
    }
  }

  override fun createSession(metadata: Map<String, String>): MobileRuntimeSession =
    parseSessionResult(runMobileCommand(MOBILE_CREATE_SESSION_PROMPT))

  override fun listResumableSessions(): List<MobileRuntimeSession> {
    val output = runMobileCommand(MOBILE_LIST_SESSIONS_PROMPT)
    return output
      .lineSequence()
      .map { it.trim() }
      .filter { it.startsWith("session=") }
      .map { line ->
        val encoded = line.substringAfter("session=")
        val parts = encoded.split('|')
        require(parts.size >= SESSION_FORMAT_MIN_PARTS) { "Malformed session list output: '$line'" }
        MobileRuntimeSession(
          id = SessionId(parts[0]),
          title = parts[1].ifBlank { null },
          isActive = parts[2].toBooleanStrictOrNull() ?: false,
        )
      }
      .toList()
  }

  override fun resumeSession(sessionId: SessionId): MobileRuntimeSession =
    parseSessionResult(runMobileCommand("$MOBILE_RESUME_SESSION_PROMPT${sessionId.value}"))

  override fun endSession(sessionId: SessionId) {
    runMobileCommand("$MOBILE_END_SESSION_PROMPT${sessionId.value}")
  }

  override fun sendMessage(sessionId: SessionId, prompt: String): MobileRuntimeTurnResult =
    runTurnCommand(
      sessionId = sessionId,
      prompt = "$MOBILE_SEND_MESSAGE_PROMPT${sessionId.value}$COMMAND_DELIMITER$prompt",
    )

  override fun submitApproval(
    requestId: String,
    decision: MobileApprovalDecision,
    sessionId: SessionId,
  ): MobileRuntimeTurnResult {
    val prompt =
      "$MOBILE_SUBMIT_APPROVAL_PROMPT${sessionId.value}" +
        "$COMMAND_DELIMITER$requestId$COMMAND_DELIMITER${decision.name}"
    return runTurnCommand(sessionId = sessionId, prompt = prompt)
  }

  private fun runMobileCommand(prompt: String): String {
    val result = invoke(CoreInvocation(prompt = prompt))
    return when (result) {
      is CoreResult.Success -> result.output.text
      is CoreResult.Failure -> throw IllegalStateException(result.details ?: result.message)
    }
  }

  private fun runTurnCommand(sessionId: SessionId, prompt: String): MobileRuntimeTurnResult {
    val result = invoke(CoreInvocation(prompt = prompt))
    return when (result) {
      is CoreResult.Success ->
        parseTurnResult(
          sessionId = sessionId,
          rawOutput = result.output.text,
          fallback = result.output.text,
        )
      is CoreResult.Failure ->
        MobileRuntimeTurnResult(
          sessionId = sessionId,
          events =
            listOf(
              MobileRuntimeEvent.Failure(
                sessionId = sessionId,
                message = result.message,
                recoverable = result.recoverable,
              )
            ),
        )
    }
  }

  private fun parseReadiness(rawOutput: String): MobileRuntimeReadinessSnapshot {
    val values = parseKeyValueOutput(rawOutput)
    return MobileRuntimeReadinessSnapshot(
      runtimeAvailable = values.booleanValue("runtime_available"),
      linkEstablished =
        values.booleanValue("link_established") || values.booleanValue("bridge_linked"),
      sessionCapable = values.booleanValue("session_capable"),
      activeSessionId = values["session_id"]?.takeIf { it.isNotBlank() }?.let(::SessionId),
      capabilities =
        MobileRuntimeCapabilities(
          streamingResponses = values.booleanValue("cap_streaming"),
          resumableSessionList = values.booleanValue("cap_resumable_sessions"),
          approvalRequests = values.booleanValue("cap_approval_requests"),
        ),
    )
  }

  private fun parseSessionResult(rawOutput: String): MobileRuntimeSession {
    val values = parseKeyValueOutput(rawOutput)
    return MobileRuntimeSession(
      id =
        SessionId(requireNotNull(values["session_id"]) { "Missing session_id in bridge output." }),
      title = values["session_title"]?.takeIf { it.isNotBlank() },
      isActive = values.booleanValue("session_active"),
    )
  }

  private fun parseTurnResult(
    sessionId: SessionId,
    rawOutput: String,
    fallback: String,
  ): MobileRuntimeTurnResult {
    val values = parseKeyValueOutput(rawOutput)
    val events = buildList {
      values["assistant_chunk"]
        ?.takeIf { it.isNotBlank() }
        ?.let { add(MobileRuntimeEvent.AssistantChunk(sessionId = sessionId, text = it)) }
      values["assistant_message"]
        ?.takeIf { it.isNotBlank() }
        ?.let { add(MobileRuntimeEvent.AssistantMessage(sessionId = sessionId, text = it)) }
      values["approval_id"]
        ?.takeIf { it.isNotBlank() }
        ?.let { approvalId ->
          add(
            MobileRuntimeEvent.ApprovalPending(
              request =
                MobileApprovalRequest(
                  id = approvalId,
                  sessionId = sessionId,
                  toolLabel = values["approval_tool"].orEmpty().ifBlank { "tool" },
                  reason =
                    values["approval_reason"].orEmpty().ifBlank { "Runtime approval required" },
                )
            )
          )
        }
      if (isEmpty() && fallback.isNotBlank()) {
        add(MobileRuntimeEvent.AssistantMessage(sessionId = sessionId, text = fallback.trim()))
      }
    }
    return MobileRuntimeTurnResult(sessionId = sessionId, events = events)
  }

  private fun parseKeyValueOutput(rawOutput: String): Map<String, String> =
    rawOutput
      .lineSequence()
      .map { it.trim() }
      .filter { it.contains('=') }
      .associate { line ->
        val separatorIndex = line.indexOf('=')
        line.substring(0, separatorIndex).trim() to line.substring(separatorIndex + 1).trim()
      }

  companion object {
    fun parseBridgeProbe(
      rawOutput: String,
      environmentSupported: Boolean = true,
    ): BridgeLinkSnapshot {
      val values =
        rawOutput
          .lineSequence()
          .map { it.trim() }
          .filter { it.contains('=') }
          .associate { line ->
            val separatorIndex = line.indexOf('=')
            line.substring(0, separatorIndex).trim() to line.substring(separatorIndex + 1).trim()
          }

      return BridgeLinkSnapshot(
        runtimeAvailable = values.booleanValue("runtime_available"),
        linkEstablished =
          values.booleanValue("link_established") || values.booleanValue("bridge_linked"),
        sessionCapable = values.booleanValue("session_capable"),
        sessionId = values["session_id"]?.takeIf { it.isNotBlank() },
        environmentSupported = environmentSupported,
      )
    }

    private const val MOBILE_PROBE_PROMPT = "__corvus_probe__"
    private const val MOBILE_CREATE_SESSION_PROMPT = "__corvus_create_session__"
    private const val MOBILE_LIST_SESSIONS_PROMPT = "__corvus_list_sessions__"
    private const val MOBILE_RESUME_SESSION_PROMPT = "__corvus_resume_session__"
    private const val MOBILE_END_SESSION_PROMPT = "__corvus_end_session__"
    private const val MOBILE_SEND_MESSAGE_PROMPT = "__corvus_send_message__"
    private const val MOBILE_SUBMIT_APPROVAL_PROMPT = "__corvus_submit_approval__"
    private const val COMMAND_DELIMITER = "\u001f"
    private const val SESSION_FORMAT_MIN_PARTS = 3

    private fun Map<String, String>.booleanValue(key: String): Boolean =
      when (this[key]?.lowercase()) {
        "1",
        "true",
        "yes",
        "ready" -> true
        else -> false
      }
  }
}
