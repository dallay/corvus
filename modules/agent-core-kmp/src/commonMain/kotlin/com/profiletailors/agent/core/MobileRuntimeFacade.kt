package com.profiletailors.agent.core

interface MobileRuntimeFacade {
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
