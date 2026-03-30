package com.profiletailors.agent.core

interface IosRuntimeCompanionClient {
  val capabilities: MobileRuntimeCapabilities

  fun probeReadiness(): MobileRuntimeReadinessSnapshot

  fun createSession(metadata: Map<String, String> = emptyMap()): MobileRuntimeSession

  fun listResumableSessions(): List<MobileRuntimeSession>

  fun resumeSession(sessionId: SessionId): MobileRuntimeSession

  fun endSession(sessionId: SessionId)

  fun sendMessage(sessionId: SessionId, prompt: String): MobileRuntimeTurnResult

  fun submitApproval(
    requestId: String,
    decision: MobileApprovalDecision,
    sessionId: SessionId,
  ): MobileRuntimeTurnResult
}

class IosRuntimeBridge(private val client: IosRuntimeCompanionClient) : MobileRuntimeFacade {
  override val capabilities: MobileRuntimeCapabilities
    get() = client.capabilities

  override fun probeReadiness(): MobileRuntimeReadinessSnapshot = client.probeReadiness()

  override fun createSession(metadata: Map<String, String>): MobileRuntimeSession =
    client.createSession(metadata)

  override fun listResumableSessions(): List<MobileRuntimeSession> = client.listResumableSessions()

  override fun resumeSession(sessionId: SessionId): MobileRuntimeSession =
    client.resumeSession(sessionId)

  override fun endSession(sessionId: SessionId) {
    client.endSession(sessionId)
  }

  override fun sendMessage(sessionId: SessionId, prompt: String): MobileRuntimeTurnResult =
    client.sendMessage(sessionId, prompt)

  override fun submitApproval(
    requestId: String,
    decision: MobileApprovalDecision,
    sessionId: SessionId,
  ): MobileRuntimeTurnResult = client.submitApproval(requestId, decision, sessionId)
}
