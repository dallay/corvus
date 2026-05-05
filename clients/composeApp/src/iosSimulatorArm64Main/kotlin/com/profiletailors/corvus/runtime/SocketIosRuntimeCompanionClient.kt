package com.profiletailors.corvus.runtime

import kotlinx.cinterop.ExperimentalForeignApi
import kotlinx.cinterop.IntVar
import kotlinx.cinterop.addressOf
import kotlinx.cinterop.alloc
import kotlinx.cinterop.convert
import kotlinx.cinterop.memScoped
import kotlinx.cinterop.ptr
import kotlinx.cinterop.reinterpret
import kotlinx.cinterop.sizeOf
import kotlinx.cinterop.usePinned
import kotlinx.cinterop.value
import platform.posix.AF_INET
import platform.posix.IPPROTO_TCP
import platform.posix.SOCK_STREAM
import platform.posix.SOL_SOCKET
import platform.posix.SO_NOSIGPIPE
import platform.posix.SO_RCVTIMEO
import platform.posix.SO_SNDTIMEO
import platform.posix.close
import platform.posix.connect
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
private const val BYTE_SHIFT = 8
private const val BYTE_MASK = 0xFF
private const val MILLIS_PER_SECOND = 1000
private const val IPV4_PART_COUNT = 4
private const val IPV4_FIRST_OCTET = 0
private const val IPV4_SECOND_OCTET = 1
private const val IPV4_THIRD_OCTET = 2
private const val IPV4_FOURTH_OCTET = 3
private const val IPV4_BYTE_MIN = 0
private const val IPV4_BYTE_MAX = 255
private const val IPV4_SHIFT_24 = 24
private const val IPV4_SHIFT_16 = 16
private const val INVALID_IPV4_ADDRESS = 0u

// Helper function to convert host byte order to network byte order (big-endian)
@OptIn(ExperimentalForeignApi::class)
private fun htons(value: UShort): UShort {
  val bytes = value.toInt()
  return ((bytes shr BYTE_SHIFT) or ((bytes and BYTE_MASK) shl BYTE_SHIFT)).toUShort()
}

private fun parseIpv4Address(host: String): UInt? {
  val bytes = host.split('.').map { it.toIntOrNull() }
  val validBytes = bytes.filterNotNull()
  val isValidAddress =
    bytes.size == IPV4_PART_COUNT &&
      validBytes.size == IPV4_PART_COUNT &&
      validBytes.all { it in IPV4_BYTE_MIN..IPV4_BYTE_MAX }

  if (!isValidAddress) {
    return null
  }

  val firstOctet = validBytes[IPV4_FIRST_OCTET].toUInt() shl IPV4_SHIFT_24
  val secondOctet = validBytes[IPV4_SECOND_OCTET].toUInt() shl IPV4_SHIFT_16
  val thirdOctet = validBytes[IPV4_THIRD_OCTET].toUInt() shl BYTE_SHIFT
  val fourthOctet = validBytes[IPV4_FOURTH_OCTET].toUInt()
  val address = firstOctet or secondOctet or thirdOctet or fourthOctet

  return address.takeUnless { it == INVALID_IPV4_ADDRESS }
}

actual data class IosRuntimeCompanionConfig
actual constructor(actual val host: String, actual val port: Int, actual val timeoutMs: Long) {
  actual companion object {
    actual val DEFAULT_COMPANION_PORT = 50_400
  }
}

@Suppress(
  "TooManyFunctions", // Implements full IosRuntimeCompanionClient; split would be artificial
  "TooGenericExceptionCaught", // Socket I/O errors must be caught broadly at the boundary
)
actual class SocketIosRuntimeCompanionClient
actual constructor(private val config: IosRuntimeCompanionConfig) : IosRuntimeCompanionClient {
  actual override val capabilities: RuntimeCapabilities =
    RuntimeCapabilities(
      streamingResponses = false,
      resumableSessionList = true,
      approvalRequests = true,
    )

  actual override fun probeReadiness(): RuntimeReadinessSnapshot =
    sendCommand("__corvus_probe__")?.let { parseReadiness(it) }
      ?: RuntimeReadinessSnapshot(
        runtimeAvailable = false,
        linkEstablished = false,
        sessionCapable = false,
        environmentSupported = true,
        capabilities = capabilities,
      )

  actual override fun createSession(metadata: Map<String, String>): RuntimeSession =
    sessionFrom(
      requireNotNull(sendCommand("__corvus_create_session__")) {
        "sendCommand('__corvus_create_session__') returned null in createSession"
      }
    )

  actual override fun listResumableSessions(): List<RuntimeSession> =
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
      ?.toList() ?: emptyList()

  actual override fun resumeSession(sessionId: RuntimeSessionId): RuntimeSession {
    val command = "__corvus_resume_session__${sessionId.value}"
    return sessionFrom(
      requireNotNull(sendCommand(command)) {
        "sendCommand('$command') returned null in resumeSession(sessionId=$sessionId)"
      }
    )
  }

  actual override fun endSession(sessionId: RuntimeSessionId) {
    val result = sendCommand("__corvus_end_session__${sessionId.value}")
    if (result == null) {
      error("Failed to end session ${sessionId.value}: iOS companion unavailable")
    }
  }

  actual override fun sendMessage(sessionId: RuntimeSessionId, prompt: String): RuntimeTurnResult =
    turnFrom(
      sessionId,
      sendCommand("__corvus_send_message__${sessionId.value}$UNIT_SEPARATOR$prompt"),
      "",
    )

  actual override fun submitApproval(
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

  @OptIn(ExperimentalForeignApi::class)
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
      tv.tv_sec = (config.timeoutMs / MILLIS_PER_SECOND).convert()
      tv.tv_usec = ((config.timeoutMs % MILLIS_PER_SECOND).toInt() * MILLIS_PER_SECOND).convert()
      setsockopt(sockfd, SOL_SOCKET, SO_RCVTIMEO, tv.ptr, sizeOf<timeval>().convert())
      setsockopt(sockfd, SOL_SOCKET, SO_SNDTIMEO, tv.ptr, sizeOf<timeval>().convert())

      val addr = alloc<sockaddr_in>()
      addr.sin_family = AF_INET.convert()
      addr.sin_port = htons(config.port.toUShort())
      addr.sin_addr.s_addr = parseIpv4Address(config.host) ?: return null

      if (connect(sockfd, addr.ptr.reinterpret(), sizeOf<sockaddr_in>().convert()) < 0) {
        return null
      }

      val payload = "$command\n".encodeToByteArray()
      val sent = payload.usePinned { send(sockfd, it.addressOf(0), payload.size.convert(), 0) }
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
