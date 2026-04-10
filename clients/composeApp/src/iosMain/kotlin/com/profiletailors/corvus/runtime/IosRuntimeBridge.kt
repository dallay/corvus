package com.profiletailors.corvus.runtime

internal val IOS_COMPANION_CLIENT_UNAVAILABLE = iosCompanionUnavailableMessage()

internal fun interface IosRuntimeCompanionClientProvider {
  fun current(): IosRuntimeCompanionClient?
}

internal object RegisteredIosRuntimeCompanionClientProvider : IosRuntimeCompanionClientProvider {
  private var installedClient: IosRuntimeCompanionClient? = null

  override fun current(): IosRuntimeCompanionClient? = installedClient

  fun install(client: IosRuntimeCompanionClient) {
    installedClient = client
  }

  fun clear() {
    installedClient = null
  }
}

fun installIosRuntimeCompanionClient(client: IosRuntimeCompanionClient) {
  RegisteredIosRuntimeCompanionClientProvider.install(client)
}

fun clearInstalledIosRuntimeCompanionClient() {
  RegisteredIosRuntimeCompanionClientProvider.clear()
}

interface IosRuntimeCompanionClient {
  val capabilities: RuntimeCapabilities

  fun probeReadiness(): RuntimeReadinessSnapshot

  fun createSession(metadata: Map<String, String> = emptyMap()): RuntimeSession

  fun listResumableSessions(): List<RuntimeSession>

  fun resumeSession(sessionId: RuntimeSessionId): RuntimeSession

  fun endSession(sessionId: RuntimeSessionId)

  fun sendMessage(sessionId: RuntimeSessionId, prompt: String): RuntimeTurnResult

  fun submitApproval(
    requestId: String,
    decision: RuntimeApprovalDecision,
    sessionId: RuntimeSessionId,
  ): RuntimeTurnResult
}

internal class MissingInfrastructureIosRuntimeCompanionClient(
  private val unavailableReason: String = IOS_COMPANION_CLIENT_UNAVAILABLE,
  private val environmentSupported: Boolean = false,
) : IosRuntimeCompanionClient {
  override val capabilities: RuntimeCapabilities = RuntimeCapabilities()

  override fun probeReadiness(): RuntimeReadinessSnapshot =
    RuntimeReadinessSnapshot(
      runtimeAvailable = false,
      linkEstablished = false,
      sessionCapable = false,
      environmentSupported = environmentSupported,
      capabilities = capabilities,
    )

  override fun createSession(metadata: Map<String, String>): RuntimeSession = unavailable()

  override fun listResumableSessions(): List<RuntimeSession> = emptyList()

  override fun resumeSession(sessionId: RuntimeSessionId): RuntimeSession = unavailable()

  override fun endSession(sessionId: RuntimeSessionId) {
    unavailable<Unit>()
  }

  override fun sendMessage(sessionId: RuntimeSessionId, prompt: String): RuntimeTurnResult =
    unavailableTurn(sessionId)

  override fun submitApproval(
    requestId: String,
    decision: RuntimeApprovalDecision,
    sessionId: RuntimeSessionId,
  ): RuntimeTurnResult = unavailableTurn(sessionId)

  private fun unavailableTurn(sessionId: RuntimeSessionId) =
    RuntimeTurnResult(
      sessionId = sessionId,
      events =
        listOf(
          RuntimeEvent.Failure(
            sessionId = sessionId,
            message = unavailableReason,
            recoverable = environmentSupported,
          )
        ),
    )

  private fun <T> unavailable(): T = throw IllegalStateException(unavailableReason)
}

class IosRuntimeBridge(private val client: IosRuntimeCompanionClient) : RuntimeFacade {
  override val capabilities: RuntimeCapabilities
    get() = client.capabilities

  override fun probeReadiness(): RuntimeReadinessSnapshot = client.probeReadiness()

  override fun createSession(metadata: Map<String, String>): RuntimeSession =
    client.createSession(metadata)

  override fun listResumableSessions(): List<RuntimeSession> = client.listResumableSessions()

  override fun resumeSession(sessionId: RuntimeSessionId): RuntimeSession =
    client.resumeSession(sessionId)

  override fun endSession(sessionId: RuntimeSessionId) {
    client.endSession(sessionId)
  }

  override fun sendMessage(sessionId: RuntimeSessionId, prompt: String): RuntimeTurnResult =
    client.sendMessage(sessionId, prompt)

  override fun submitApproval(
    requestId: String,
    decision: RuntimeApprovalDecision,
    sessionId: RuntimeSessionId,
  ): RuntimeTurnResult = client.submitApproval(requestId, decision, sessionId)
}
