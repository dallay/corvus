@file:Suppress(
  "MatchingDeclarationName", // File name matches the expect declaration, not the actual
  "TooManyFunctions", // File implements full RuntimeFacade; functions belong together by design
)

package com.profiletailors.corvus.runtime

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import com.profiletailors.agent.core.MobileApprovalDecision
import com.profiletailors.agent.core.MobileRuntimeEvent
import com.profiletailors.agent.core.RustCliBridge
import com.profiletailors.agent.core.SessionId
import com.profiletailors.corvus.ui.chat.MobileBridgeSnapshot

// Client-first: JVM should NOT default to RustCliBridge. Instead, it requires
// explicit endpoint configuration or trusted companion setup before becoming ready.
// This implements the client-first onboarding model per the delta specs.
internal class JvmRuntimeFacade(private val delegate: RustCliBridge? = null) : RuntimeFacade {
  override val capabilities: RuntimeCapabilities
    get() = delegate?.capabilities?.toLocal() ?: RuntimeCapabilities()

  override fun probeReadiness(): RuntimeReadinessSnapshot {
    // Client-first: If no explicit bridge configured, fail closed into onboarding
    val delegate = this.delegate
    return if (delegate == null) {
      RuntimeReadinessSnapshot(
        runtimeAvailable = false,
        linkEstablished = false,
        sessionCapable = false,
        environmentSupported = true,
        capabilities = RuntimeCapabilities(),
      )
    } else {
      delegate.probeReadiness().toLocal()
    }
  }

  override fun createSession(metadata: Map<String, String>): RuntimeSession {
    // Client-first: Block session creation if no runtime configured
    check(delegate != null) {
      "Cannot create session: No runtime endpoint configured. Please configure a runtime connection first."
    }
    return delegate.createSession(metadata).toLocal()
  }

  override fun listResumableSessions(): List<RuntimeSession> {
    if (delegate == null) return emptyList()
    return delegate.listResumableSessions().map { it.toLocal() }
  }

  override fun resumeSession(sessionId: RuntimeSessionId): RuntimeSession {
    check(delegate != null) {
      "Cannot resume session: No runtime endpoint configured. Please configure a runtime connection first."
    }
    return delegate.resumeSession(sessionId.toCore()).toLocal()
  }

  override fun endSession(sessionId: RuntimeSessionId) {
    delegate?.endSession(sessionId.toCore())
  }

  override fun sendMessage(sessionId: RuntimeSessionId, prompt: String): RuntimeTurnResult {
    check(delegate != null) {
      "Cannot send message: No runtime endpoint configured. Please configure a runtime connection first."
    }
    return delegate.sendMessage(sessionId.toCore(), prompt).toLocal()
  }

  override fun submitApproval(
    requestId: String,
    decision: RuntimeApprovalDecision,
    sessionId: RuntimeSessionId,
  ): RuntimeTurnResult {
    check(delegate != null) {
      "Cannot submit approval: No runtime endpoint configured. Please configure a runtime connection first."
    }
    return delegate.submitApproval(requestId, decision.toCore(), sessionId.toCore()).toLocal()
  }
}

// JVM supported connection methods for client-first model
internal val jvmSupportedConnectionMethods: Set<RuntimeConnectionMethod> =
  setOf(RuntimeConnectionMethod.ENDPOINT_URL, RuntimeConnectionMethod.LOCAL_HOST_ADVANCED)

// Expected declaration for common code
actual val platformSupportedConnectionMethods: Set<RuntimeConnectionMethod>
  get() = jvmSupportedConnectionMethods

internal fun createJvmRuntimeFacade(initialBridgeSnapshot: MobileBridgeSnapshot?): RuntimeFacade =
  initialBridgeSnapshot?.let(::PreviewMobileRuntimeFacade) ?: JvmRuntimeFacade()

internal fun createJvmRuntimePersistence(): MobileRuntimePersistence =
  InMemoryMobileRuntimePersistence()

@Composable
actual fun rememberPlatformRuntimeDependencies(
  initialBridgeSnapshot: MobileBridgeSnapshot?
): PlatformRuntimeDependencies {
  val facade = remember(initialBridgeSnapshot) { createJvmRuntimeFacade(initialBridgeSnapshot) }
  val persistence = remember { createJvmRuntimePersistence() }
  return remember(facade, persistence) { PlatformRuntimeDependencies(facade, persistence) }
}

private fun RuntimeSessionId.toCore(): SessionId = SessionId(value)

private fun RuntimeApprovalDecision.toCore(): MobileApprovalDecision =
  when (this) {
    RuntimeApprovalDecision.APPROVE -> MobileApprovalDecision.APPROVE
    RuntimeApprovalDecision.DENY -> MobileApprovalDecision.DENY
  }

private fun com.profiletailors.agent.core.MobileRuntimeCapabilities.toLocal(): RuntimeCapabilities =
  RuntimeCapabilities(
    streamingResponses = streamingResponses,
    resumableSessionList = resumableSessionList,
    approvalRequests = approvalRequests,
  )

private fun com.profiletailors.agent.core.MobileRuntimeReadinessSnapshot.toLocal():
  RuntimeReadinessSnapshot =
  RuntimeReadinessSnapshot(
    runtimeAvailable = runtimeAvailable,
    linkEstablished = linkEstablished,
    sessionCapable = sessionCapable,
    activeSessionId = activeSessionId?.let { RuntimeSessionId(it.value) },
    environmentSupported = environmentSupported,
    capabilities = capabilities.toLocal(),
  )

private fun com.profiletailors.agent.core.MobileRuntimeSession.toLocal(): RuntimeSession =
  RuntimeSession(id = RuntimeSessionId(id.value), title = title, isActive = isActive)

private fun com.profiletailors.agent.core.MobileApprovalRequest.toLocal(): RuntimeApprovalRequest =
  RuntimeApprovalRequest(
    id = id,
    sessionId = RuntimeSessionId(sessionId.value),
    toolLabel = toolLabel,
    reason = reason,
  )

private fun com.profiletailors.agent.core.MobileRuntimeTurnResult.toLocal(): RuntimeTurnResult =
  RuntimeTurnResult(
    sessionId = RuntimeSessionId(sessionId.value),
    events = events.map { it.toLocal() },
  )

private fun MobileRuntimeEvent.toLocal(): RuntimeEvent =
  when (this) {
    is MobileRuntimeEvent.AssistantChunk ->
      RuntimeEvent.AssistantChunk(sessionId = RuntimeSessionId(sessionId.value), text = text)
    is MobileRuntimeEvent.AssistantMessage ->
      RuntimeEvent.AssistantMessage(sessionId = RuntimeSessionId(sessionId.value), text = text)
    is MobileRuntimeEvent.ApprovalPending -> RuntimeEvent.ApprovalPending(request.toLocal())
    is MobileRuntimeEvent.Failure ->
      RuntimeEvent.Failure(
        sessionId = RuntimeSessionId(sessionId.value),
        message = message,
        recoverable = recoverable,
      )
  }
