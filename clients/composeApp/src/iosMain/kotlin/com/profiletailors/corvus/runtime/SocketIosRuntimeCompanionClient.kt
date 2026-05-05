package com.profiletailors.corvus.runtime

// Type alias to the actual platform-specific implementation.
// The actual implementation is in iosArm64Main and iosSimulatorArm64Main.
expect class SocketIosRuntimeCompanionClient(config: IosRuntimeCompanionConfig) :
  IosRuntimeCompanionClient {
  override val capabilities: RuntimeCapabilities

  override fun probeReadiness(): RuntimeReadinessSnapshot

  override fun createSession(metadata: Map<String, String>): RuntimeSession

  override fun listResumableSessions(): List<RuntimeSession>

  override fun resumeSession(sessionId: RuntimeSessionId): RuntimeSession

  override fun endSession(sessionId: RuntimeSessionId)

  override fun sendMessage(sessionId: RuntimeSessionId, prompt: String): RuntimeTurnResult

  override fun submitApproval(
    requestId: String,
    decision: RuntimeApprovalDecision,
    sessionId: RuntimeSessionId,
  ): RuntimeTurnResult
}

expect class IosRuntimeCompanionConfig(host: String, port: Int, timeoutMs: Long) {
  companion object {
    val DEFAULT_COMPANION_PORT: Int
  }

  val host: String
  val port: Int
  val timeoutMs: Long
}
