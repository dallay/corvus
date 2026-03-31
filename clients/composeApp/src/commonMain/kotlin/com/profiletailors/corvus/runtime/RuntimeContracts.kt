package com.profiletailors.corvus.runtime

import com.profiletailors.corvus.ui.chat.MobileBridgeSnapshot
import com.profiletailors.corvus.ui.chat.MobileOnboardingState
import com.profiletailors.corvus.ui.chat.MobileOnboardingStatus
import com.profiletailors.corvus.ui.chat.MobileRecoveryKind
import com.profiletailors.corvus.ui.chat.MobileTransportMode
import com.profiletailors.corvus.ui.chat.MobileTrustMode
import kotlin.jvm.JvmInline

private val uuidPattern =
  Regex("^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$")

@JvmInline
value class RuntimeSessionId(val value: String) {
  init {
    require(uuidPattern.matches(value)) { "Session identifiers must be UUID-based." }
  }
}

enum class RuntimeTrustMode {
  HOST_TRUSTED,
  HTTP_PAIRED,
  BRIDGE_LINKED,
  // Client-first trust modes (Task 1.2)
  PAIRING_REQUIRED,
  TRUSTED_COMPANION_ESTABLISHED,
}

enum class RuntimeTransportMode {
  DIRECT,
  HTTP_GATEWAY,
  CLI_BRIDGE,
  // Client-first connection methods (Task 1.2)
  ENDPOINT_URL,
  TRUSTED_COMPANION,
  LOCAL_HOST_ADVANCED,
}

// Client-first connection method enum for composeApp
enum class RuntimeConnectionMethod {
  ENDPOINT_URL,
  TRUSTED_COMPANION,
  LOCAL_HOST_ADVANCED,
}

// Client-first connection target
data class RuntimeConnectionTarget(
  val id: String,
  val label: String,
  val method: RuntimeConnectionMethod,
  val endpointUrl: String? = null,
)

// Client-first trust state
data class RuntimeTrustState(val established: Boolean, val requiresPairingOrAuth: Boolean)

// Client-first readiness snapshot
data class RuntimeConnectionReadinessSnapshot(
  val target: RuntimeConnectionTarget?,
  val trustState: RuntimeTrustState,
  val transportReachable: Boolean,
  val sessionCapable: Boolean,
  val activeSessionId: RuntimeSessionId? = null,
  val supportedMethods: Set<RuntimeConnectionMethod> = emptySet(),
) {
  val isReadyForChat: Boolean
    get() =
      target != null &&
        trustState.established &&
        transportReachable &&
        sessionCapable &&
        (activeSessionId != null || supportedMethods.isNotEmpty())
}

enum class RuntimeRecoveryKind {
  RUNTIME_UNAVAILABLE,
  TRANSPORT_UNAVAILABLE,
  TRUST_INPUT_INVALID,
  TRUST_INPUT_EXPIRED,
  CREDENTIAL_MISSING,
  CREDENTIAL_INVALID,
  PAIRED_BUT_NOT_CONNECTED,
  LINKED_BUT_NOT_SESSION_READY,
  SESSION_UNAVAILABLE,
  ENVIRONMENT_UNSUPPORTED,
  // Client-first recovery kinds (Task 1.2)
  NO_TARGET_CONFIGURED,
  TARGET_NOT_REACHABLE,
  TRUST_NOT_ESTABLISHED,
}

data class RuntimeOnboardingState(
  val status: Status,
  val trustMode: RuntimeTrustMode,
  val transportMode: RuntimeTransportMode,
  val recoveryKind: RuntimeRecoveryKind? = null,
  val canRetry: Boolean = false,
  val canResume: Boolean = false,
) {
  init {
    require(status != Status.BLOCKED || recoveryKind != null) {
      "Blocked onboarding states require a recovery kind"
    }
    require(status == Status.BLOCKED || recoveryKind == null) {
      "Recovery kind is only valid for blocked onboarding states"
    }
  }

  enum class Status {
    // Client-first onboarding states (Task 1.2)
    TARGET_SELECTED, // User has selected a connection target
    RECOVERY, // Recovery action needed

    // Legacy states (kept for compatibility)
    INTENT_SELECTED,
    RUNTIME_PATH_CONFIRMED,
    TRUST_PENDING,
    TRUST_ESTABLISHED,
    TRANSPORT_CONNECTING,
    READY,
    SESSION_PENDING,
    SESSION_READY,
    BLOCKED,
  }
}

data class RuntimeCapabilities(
  val streamingResponses: Boolean = false,
  val resumableSessionList: Boolean = false,
  val approvalRequests: Boolean = false,
)

data class RuntimeReadinessSnapshot(
  val runtimeAvailable: Boolean,
  val linkEstablished: Boolean,
  val sessionCapable: Boolean,
  val activeSessionId: RuntimeSessionId? = null,
  val environmentSupported: Boolean = true,
  val capabilities: RuntimeCapabilities = RuntimeCapabilities(),
) {
  fun toOnboardingState(): RuntimeOnboardingState =
    when {
      !environmentSupported ->
        blockedState(recoveryKind = RuntimeRecoveryKind.ENVIRONMENT_UNSUPPORTED, canRetry = false)

      !runtimeAvailable -> blockedState(recoveryKind = RuntimeRecoveryKind.RUNTIME_UNAVAILABLE)
      !linkEstablished ->
        RuntimeOnboardingState(
          status = RuntimeOnboardingState.Status.TRUST_PENDING,
          trustMode = RuntimeTrustMode.BRIDGE_LINKED,
          transportMode = RuntimeTransportMode.CLI_BRIDGE,
          canRetry = true,
        )

      !sessionCapable ->
        blockedState(
          recoveryKind = RuntimeRecoveryKind.LINKED_BUT_NOT_SESSION_READY,
          canResume = activeSessionId != null,
        )

      activeSessionId != null ->
        RuntimeOnboardingState(
          status = RuntimeOnboardingState.Status.SESSION_READY,
          trustMode = RuntimeTrustMode.BRIDGE_LINKED,
          transportMode = RuntimeTransportMode.CLI_BRIDGE,
          canRetry = true,
          canResume = true,
        )

      else ->
        RuntimeOnboardingState(
          status = RuntimeOnboardingState.Status.SESSION_PENDING,
          trustMode = RuntimeTrustMode.BRIDGE_LINKED,
          transportMode = RuntimeTransportMode.CLI_BRIDGE,
          canRetry = true,
        )
    }

  private fun blockedState(
    recoveryKind: RuntimeRecoveryKind,
    canRetry: Boolean = true,
    canResume: Boolean = false,
  ) =
    RuntimeOnboardingState(
      status = RuntimeOnboardingState.Status.BLOCKED,
      trustMode = RuntimeTrustMode.BRIDGE_LINKED,
      transportMode = RuntimeTransportMode.CLI_BRIDGE,
      recoveryKind = recoveryKind,
      canRetry = canRetry,
      canResume = canResume,
    )
}

data class RuntimeSession(
  val id: RuntimeSessionId,
  val title: String? = null,
  val isActive: Boolean = false,
)

data class RuntimeApprovalRequest(
  val id: String,
  val sessionId: RuntimeSessionId,
  val toolLabel: String,
  val reason: String,
) {
  init {
    require(id.isNotBlank()) { "Approval request identifiers must not be blank." }
    require(toolLabel.isNotBlank()) { "Approval tool labels must not be blank." }
    require(reason.isNotBlank()) { "Approval reasons must not be blank." }
  }
}

enum class RuntimeApprovalDecision {
  APPROVE,
  DENY,
}

sealed interface RuntimeEvent {
  data class AssistantChunk(val sessionId: RuntimeSessionId, val text: String) : RuntimeEvent

  data class AssistantMessage(val sessionId: RuntimeSessionId, val text: String) : RuntimeEvent

  data class ApprovalPending(val request: RuntimeApprovalRequest) : RuntimeEvent

  data class Failure(
    val sessionId: RuntimeSessionId,
    val message: String,
    val recoverable: Boolean,
  ) : RuntimeEvent
}

data class RuntimeTurnResult(val sessionId: RuntimeSessionId, val events: List<RuntimeEvent>)

data class LinkedRuntimeMetadata(
  val targetId: String,
  val transportMode: RuntimeTransportMode = RuntimeTransportMode.CLI_BRIDGE,
  val trustMode: RuntimeTrustMode = RuntimeTrustMode.BRIDGE_LINKED,
  val linkedAtEpochMs: Long? = null,
  // Client-first fields (Task 1.2)
  val connectionMethod: RuntimeConnectionMethod? = null,
  val endpointUrl: String? = null,
) {
  init {
    require(targetId.isNotBlank()) { "Runtime target identifiers must not be blank." }
  }
}

interface MobileRuntimeFacade {
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

class FailClosedRuntimeFacade(
  private val unavailableReason: String,
  private val environmentSupported: Boolean = true,
  override val capabilities: RuntimeCapabilities = RuntimeCapabilities(),
) : MobileRuntimeFacade {
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

interface MobileRuntimePersistence {
  fun readLinkedRuntimeMetadata(): LinkedRuntimeMetadata?

  fun saveLinkedRuntimeMetadata(metadata: LinkedRuntimeMetadata)

  fun clearLinkedRuntimeMetadata()

  fun readActiveSessionId(): RuntimeSessionId?

  fun saveActiveSessionId(sessionId: RuntimeSessionId)

  fun clearActiveSessionId()
}

typealias RuntimeFacade = MobileRuntimeFacade

internal fun MobileBridgeSnapshot.toRuntimeReadinessSnapshot(): RuntimeReadinessSnapshot =
  RuntimeReadinessSnapshot(
    runtimeAvailable = runtimeAvailable,
    linkEstablished = linkEstablished,
    sessionCapable = sessionCapable,
    activeSessionId = sessionId?.takeIf { it.isNotBlank() }?.let(::RuntimeSessionId),
    environmentSupported = environmentSupported,
    capabilities = RuntimeCapabilities(),
  )

internal fun MobileBridgeSnapshot.toCoreOnboardingState(): RuntimeOnboardingState =
  recoveryOverride?.let { recoveryKind ->
    RuntimeOnboardingState(
      status = RuntimeOnboardingState.Status.BLOCKED,
      trustMode = RuntimeTrustMode.BRIDGE_LINKED,
      transportMode = RuntimeTransportMode.CLI_BRIDGE,
      recoveryKind = recoveryKind.toRuntimeRecoveryKind(),
      canRetry = recoveryKind != MobileRecoveryKind.ENVIRONMENT_UNSUPPORTED,
      canResume =
        !sessionId.isNullOrBlank() && recoveryKind == MobileRecoveryKind.SESSION_UNAVAILABLE,
    )
  } ?: toRuntimeReadinessSnapshot().toOnboardingState()

internal fun RuntimeOnboardingState.toMobileOnboardingState(): MobileOnboardingState =
  MobileOnboardingState(
    status = status.toMobileOnboardingStatus(),
    trustMode = trustMode.toMobileTrustMode(),
    transportMode = transportMode.toMobileTransportMode(),
    recoveryKind = recoveryKind?.toMobileRecoveryKind(),
    canRetry = canRetry,
    canResume = canResume,
  )

private fun RuntimeOnboardingState.Status.toMobileOnboardingStatus(): MobileOnboardingStatus =
  when (this) {
    RuntimeOnboardingState.Status.TARGET_SELECTED -> MobileOnboardingStatus.TARGET_SELECTED
    RuntimeOnboardingState.Status.RECOVERY -> MobileOnboardingStatus.RECOVERY
    RuntimeOnboardingState.Status.INTENT_SELECTED,
    RuntimeOnboardingState.Status.RUNTIME_PATH_CONFIRMED ->
      MobileOnboardingStatus.RUNTIME_PATH_CONFIRMED
    RuntimeOnboardingState.Status.TRUST_PENDING,
    RuntimeOnboardingState.Status.TRUST_ESTABLISHED -> MobileOnboardingStatus.TRUST_PENDING
    RuntimeOnboardingState.Status.TRANSPORT_CONNECTING ->
      MobileOnboardingStatus.TRANSPORT_CONNECTING
    RuntimeOnboardingState.Status.READY,
    RuntimeOnboardingState.Status.SESSION_PENDING -> MobileOnboardingStatus.SESSION_PENDING
    RuntimeOnboardingState.Status.SESSION_READY -> MobileOnboardingStatus.SESSION_READY
    RuntimeOnboardingState.Status.BLOCKED -> MobileOnboardingStatus.BLOCKED
  }

private fun RuntimeTrustMode.toMobileTrustMode(): MobileTrustMode =
  when (this) {
    RuntimeTrustMode.BRIDGE_LINKED -> MobileTrustMode.BRIDGE_LINKED
    RuntimeTrustMode.PAIRING_REQUIRED -> MobileTrustMode.PAIRING_REQUIRED
    RuntimeTrustMode.TRUSTED_COMPANION_ESTABLISHED -> MobileTrustMode.TRUSTED_COMPANION_ESTABLISHED
    RuntimeTrustMode.HOST_TRUSTED,
    RuntimeTrustMode.HTTP_PAIRED -> MobileTrustMode.BRIDGE_LINKED
  }

private fun RuntimeTransportMode.toMobileTransportMode(): MobileTransportMode =
  when (this) {
    RuntimeTransportMode.CLI_BRIDGE -> MobileTransportMode.CLI_BRIDGE
    RuntimeTransportMode.ENDPOINT_URL -> MobileTransportMode.ENDPOINT_URL
    RuntimeTransportMode.TRUSTED_COMPANION -> MobileTransportMode.TRUSTED_COMPANION
    RuntimeTransportMode.LOCAL_HOST_ADVANCED -> MobileTransportMode.LOCAL_HOST_ADVANCED
    RuntimeTransportMode.DIRECT,
    RuntimeTransportMode.HTTP_GATEWAY -> MobileTransportMode.CLI_BRIDGE
  }

private fun RuntimeRecoveryKind.toMobileRecoveryKind(): MobileRecoveryKind =
  when (this) {
    RuntimeRecoveryKind.NO_TARGET_CONFIGURED -> MobileRecoveryKind.NO_TARGET_CONFIGURED
    RuntimeRecoveryKind.TARGET_NOT_REACHABLE -> MobileRecoveryKind.TARGET_NOT_REACHABLE
    RuntimeRecoveryKind.TRUST_NOT_ESTABLISHED -> MobileRecoveryKind.TRUST_NOT_ESTABLISHED
    RuntimeRecoveryKind.RUNTIME_UNAVAILABLE -> MobileRecoveryKind.RUNTIME_UNAVAILABLE
    RuntimeRecoveryKind.TRANSPORT_UNAVAILABLE -> MobileRecoveryKind.TRANSPORT_UNAVAILABLE
    RuntimeRecoveryKind.LINKED_BUT_NOT_SESSION_READY ->
      MobileRecoveryKind.LINKED_BUT_NOT_SESSION_READY
    RuntimeRecoveryKind.SESSION_UNAVAILABLE -> MobileRecoveryKind.SESSION_UNAVAILABLE
    RuntimeRecoveryKind.ENVIRONMENT_UNSUPPORTED -> MobileRecoveryKind.ENVIRONMENT_UNSUPPORTED
    RuntimeRecoveryKind.TRUST_INPUT_INVALID,
    RuntimeRecoveryKind.TRUST_INPUT_EXPIRED,
    RuntimeRecoveryKind.CREDENTIAL_MISSING,
    RuntimeRecoveryKind.CREDENTIAL_INVALID,
    RuntimeRecoveryKind.PAIRED_BUT_NOT_CONNECTED -> MobileRecoveryKind.TRANSPORT_UNAVAILABLE
  }

private fun MobileRecoveryKind.toRuntimeRecoveryKind(): RuntimeRecoveryKind =
  when (this) {
    MobileRecoveryKind.NO_TARGET_CONFIGURED -> RuntimeRecoveryKind.NO_TARGET_CONFIGURED
    MobileRecoveryKind.TARGET_NOT_REACHABLE -> RuntimeRecoveryKind.TARGET_NOT_REACHABLE
    MobileRecoveryKind.TRUST_NOT_ESTABLISHED -> RuntimeRecoveryKind.TRUST_NOT_ESTABLISHED
    MobileRecoveryKind.RUNTIME_UNAVAILABLE -> RuntimeRecoveryKind.RUNTIME_UNAVAILABLE
    MobileRecoveryKind.TRANSPORT_UNAVAILABLE -> RuntimeRecoveryKind.TRANSPORT_UNAVAILABLE
    MobileRecoveryKind.LINKED_BUT_NOT_SESSION_READY ->
      RuntimeRecoveryKind.LINKED_BUT_NOT_SESSION_READY
    MobileRecoveryKind.SESSION_UNAVAILABLE -> RuntimeRecoveryKind.SESSION_UNAVAILABLE
    MobileRecoveryKind.ENVIRONMENT_UNSUPPORTED -> RuntimeRecoveryKind.ENVIRONMENT_UNSUPPORTED
  }
