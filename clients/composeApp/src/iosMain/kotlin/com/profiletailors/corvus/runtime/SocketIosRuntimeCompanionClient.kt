@file:Suppress("TooManyFunctions") // Implements full IosRuntimeCompanionClient interface; split would be artificial

package com.profiletailors.corvus.runtime

import kotlinx.cinterop.alloc
import kotlinx.cinterop.convert
import kotlinx.cinterop.memScoped
import kotlinx.cinterop.ptr
import kotlinx.cinterop.reinterpret
import kotlinx.cinterop.sizeOf
import kotlinx.cinterop.usePinned
import platform.posix.AF_INET
import platform.posix.IPPROTO_TCP
import platform.posix.IntVar
import platform.posix.SOCK_STREAM
import platform.posix.SOL_SOCKET
import platform.posix.SO_NOSIGPIPE
import platform.posix.SO_RCVTIMEO
import platform.posix.SO_SNDTIMEO
import platform.posix.close
import platform.posix.connect
import platform.posix.htons
import platform.posix.inet_pton
import platform.posix.recv
import platform.posix.send
import platform.posix.setsockopt
import platform.posix.sockaddr_in
import platform.posix.socket
import platform.posix.timeval

// Unit separator character used to delimit command parameters (matches AndroidRuntimeBridge)
private const val UNIT_SEPARATOR = '\u001f'

// Receive buffer size in bytes
private const val SOCKET_BUFFER_SIZE = 65_536

data class IosRuntimeCompanionConfig(
  val host: String = "127.0.0.1",
  val port: Int = DEFAULT_COMPANION_PORT,
  val timeoutMs: Long = 30_000L,
) {
  companion object {
    const val DEFAULT_COMPANION_PORT = 50_400
  }
}

@Suppress("TooGenericExceptionCaught") // Socket I/O errors must be caught broadly at the boundary
class SocketIosRuntimeCompanionClient(
  private val config: IosRuntimeCompanionConfig = IosRuntimeCompanionConfig()
) : IosRuntimeCompanionClient {
  override val capabilities: RuntimeCapabilities =
    RuntimeCapabilities(
      streamingResponses = false,
      resumableSessionList = true,
      approvalRequests = true,
    )

  override fun probeReadiness(): RuntimeReadinessSnapshot =
    sendCommand("__corvus_probe__")?.let { parseReadiness(it) }
      ?: RuntimeReadinessSnapshot(
        runtimeAvailable = false,
        linkEstablished = false,
        sessionCapable = false,
        environmentSupported = true,
        capabilities = capabilities,
      )

  override fun createSession(metadata: Map<String, String>): RuntimeSession =
    sessionFrom(
      requireNotNull(sendCommand("__corvus_create_session__")) {
        "sendCommand('__corvus_create_session__') returned null in createSession(metadata=$metadata)"
      }
    )

  override fun listResumableSessions(): List<RuntimeSession> =
    sendCommand("__corvus_list_sessions__")
      ?.lineSequence()
      ?.map { it.trim() }
      ?.filter { it.startsWith("session=") }
      ?.map { line ->
        val parts = line.substringAfter("session=").split('|')
        RuntimeSession(
          id = RuntimeSessionId(parts[0]),
          title = parts.getOrNull(1)?.ifBlank { null },
          isActive = parts.getOrNull(2)?.toBooleanStrictOrNull() ?: false,
        )
      }
      ?.toList()
      ?: emptyList()

  override fun resumeSession(sessionId: RuntimeSessionId): RuntimeSession =
    sessionFrom(
      requireNotNull(sendCommand("__corvus_resume_session__${sessionId.value}")) {
        "sendCommand('__corvus_resume_session__${sessionId.value}') returned null " +
          "in resumeSession(sessionId=$sessionId)"
      }
    )

  override fun endSession(sessionId: RuntimeSessionId) {
    val result = sendCommand("__corvus_end_session__${sessionId.value}")
    if (result == null) {
      error("Failed to end session ${sessionId.value}: iOS companion unavailable")
    }
  }

  override fun sendMessage(sessionId: RuntimeSessionId, prompt: String): RuntimeTurnResult =
    turnFrom(
      sessionId,
      sendCommand("__corvus_send_message__${sessionId.value}$UNIT_SEPARATOR$prompt"),
      "",
    )

  override fun submitApproval(
    requestId: String,
    decision: RuntimeApprovalDecision,
    sessionId: RuntimeSessionId,
  ): RuntimeTurnResult =
    turnFrom(
      sessionId,
      sendCommand(
        "__corvus_submit_approval__${sessionId.value}$UNIT_SEPARATOR$requestId$UNIT_SEPARATOR${decision.name}"
      ),
      "",
    )

  private fun sendCommand(command: String): String? =
    try {
      socketSendReceive(command)
    } catch (_: Exception) {
      null
    }

  private fun socketSendReceive(command: String): String? = memScoped {
    val sockfd = socket(AF_INET, SOCK_STREAM, IPPROTO_TCP)
    if (sockfd < 0) return null
    try {
      // Suppress SIGPIPE — write to a closed peer must not kill the process on Darwin/iOS
      val noSigPipe = alloc<IntVar>()
      noSigPipe.value = 1
      setsockopt(sockfd, SOL_SOCKET, SO_NOSIGPIPE, noSigPipe.ptr, sizeOf<IntVar>().convert())

      // Apply send/receive timeouts
      val tv = alloc<timeval>()
      tv.tv_sec = (config.timeoutMs / 1000).convert()
      tv.tv_usec = ((config.timeoutMs % 1000) * 1000).toInt()
      setsockopt(sockfd, SOL_SOCKET, SO_RCVTIMEO, tv.ptr, sizeOf<timeval>().convert())
      setsockopt(sockfd, SOL_SOCKET, SO_SNDTIMEO, tv.ptr, sizeOf<timeval>().convert())

      val addr = alloc<sockaddr_in>()
      addr.sin_family = AF_INET.convert()
      addr.sin_port = htons(config.port.toUShort())
      inet_pton(AF_INET, config.host, addr.sin_addr.ptr)

      if (connect(sockfd, addr.ptr.reinterpret(), sizeOf<sockaddr_in>().convert()) < 0) {
        return null
      }

      val payload = "$command\n".encodeToByteArray()
      val sent =
        payload.usePinned {
          send(sockfd, it.addressOf(0), payload.size.convert(), 0)
        }
      if (sent < 0) return null

      val buf = ByteArray(SOCKET_BUFFER_SIZE)
      val sb = StringBuilder()
      var bytesRead: Long
      do {
        bytesRead = buf.usePinned { recv(sockfd, it.addressOf(0), buf.size.convert(), 0) }
        if (bytesRead > 0) sb.append(buf.decodeToString(0, bytesRead.toInt()))
      } while (bytesRead > 0)

      sb.toString().trim().takeIf { it.isNotEmpty() }
    } finally {
      close(sockfd)
    }
  }

  private fun parseReadiness(raw: String): RuntimeReadinessSnapshot {
    val values = raw.toValues()
    val hasCapabilities =
      values["cap_streaming"] != null ||
        values["cap_resumable_sessions"] != null ||
        values["cap_approval_requests"] != null
    val probedCapabilities =
      if (hasCapabilities) {
        RuntimeCapabilities(
          streamingResponses = values.booleanValue("cap_streaming"),
          resumableSessionList = values.booleanValue("cap_resumable_sessions"),
          approvalRequests = values.booleanValue("cap_approval_requests"),
        )
      } else {
        capabilities
      }
    return RuntimeReadinessSnapshot(
      runtimeAvailable = values.booleanValue("runtime_available"),
      linkEstablished =
        values.booleanValue("link_established") || values.booleanValue("bridge_linked"),
      sessionCapable = values.booleanValue("session_capable"),
      activeSessionId = values["session_id"]?.takeIf { it.isNotBlank() }?.let(::RuntimeSessionId),
      environmentSupported = true,
      capabilities = probedCapabilities,
    )
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
            message = "iOS companion transport unavailable.",
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
