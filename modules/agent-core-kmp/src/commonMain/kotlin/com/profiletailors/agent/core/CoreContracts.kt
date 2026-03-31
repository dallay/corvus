package com.profiletailors.agent.core

private val UUID_PATTERN =
  Regex("^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$")

data class CoreInvocation(
  val prompt: String,
  val sessionId: String? = null,
  val metadata: Map<String, String> = emptyMap(),
  val timeoutMs: Long? = null,
)

data class CoreOutput(val text: String, val transport: String, val rawOutput: String? = null)

@JvmInline
value class SessionId(val value: String) {
  init {
    require(UUID_PATTERN.matches(value)) { "Session identifiers must be UUID-based." }
  }
}

enum class SurfaceId {
  CLI_RUNTIME,
  WEB_DASHBOARD,
  WEB_CHAT,
  COMPOSEAPP_MOBILE,
}

enum class TrustMode {
  HOST_TRUSTED,
  HTTP_PAIRED,
  BRIDGE_LINKED,
}

enum class TransportMode {
  DIRECT,
  HTTP_GATEWAY,
  CLI_BRIDGE,
}

// Client-first connection methods (Task 1.2)
enum class RuntimeConnectionMethod {
  ENDPOINT_URL,
  TRUSTED_COMPANION,
  LOCAL_HOST_ADVANCED,
}

enum class RecoveryKind {
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
}

val RecoveryKind.label: String
  get() = name.lowercase()

val OnboardingState.Status.label: String
  get() = name.lowercase()

fun onboardingTransitionLabel(from: OnboardingState.Status, to: OnboardingState.Status): String =
  "${from.label}__to__${to.label}"

data class OnboardingState(
  val surfaceId: SurfaceId,
  val state: Status,
  val trustMode: TrustMode,
  val transportMode: TransportMode,
  val recoveryKind: RecoveryKind? = null,
  val canRetry: Boolean = false,
  val canResume: Boolean = false,
) {
  init {
    require(state != Status.BLOCKED || recoveryKind != null) {
      "Blocked onboarding states require a recovery kind"
    }
    require(state == Status.BLOCKED || recoveryKind == null) {
      "Recovery kind is only valid for blocked onboarding states"
    }
  }

  val isReadyForPrimaryTask: Boolean
    get() = state == Status.READY || requiresSessionEntry

  val requiresSessionEntry: Boolean
    get() = state == Status.SESSION_PENDING || state == Status.SESSION_READY

  val stateLabel: String
    get() = state.label

  val recoveryLabel: String?
    get() = recoveryKind?.label

  enum class Status {
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

data class BridgeLinkSnapshot(
  val runtimeAvailable: Boolean,
  val linkEstablished: Boolean,
  val sessionCapable: Boolean,
  val sessionId: String? = null,
  val environmentSupported: Boolean = true,
) {
  fun toOnboardingState(surfaceId: SurfaceId = SurfaceId.COMPOSEAPP_MOBILE): OnboardingState {
    require(surfaceId == SurfaceId.COMPOSEAPP_MOBILE) {
      "BridgeLinkSnapshot only supports the ${SurfaceId.COMPOSEAPP_MOBILE} surface contract"
    }

    return when {
      !environmentSupported ->
        blockedState(
          surfaceId = surfaceId,
          recoveryKind = RecoveryKind.ENVIRONMENT_UNSUPPORTED,
          canRetry = false,
        )

      !runtimeAvailable ->
        blockedState(surfaceId = surfaceId, recoveryKind = RecoveryKind.RUNTIME_UNAVAILABLE)

      !linkEstablished ->
        OnboardingState(
          surfaceId = surfaceId,
          state = OnboardingState.Status.TRUST_PENDING,
          trustMode = TrustMode.BRIDGE_LINKED,
          transportMode = TransportMode.CLI_BRIDGE,
          canRetry = true,
        )

      !sessionCapable ->
        blockedState(
          surfaceId = surfaceId,
          recoveryKind = RecoveryKind.LINKED_BUT_NOT_SESSION_READY,
          canResume = !sessionId.isNullOrBlank(),
        )

      !sessionId.isNullOrBlank() ->
        OnboardingState(
          surfaceId = surfaceId,
          state = OnboardingState.Status.SESSION_READY,
          trustMode = TrustMode.BRIDGE_LINKED,
          transportMode = TransportMode.CLI_BRIDGE,
          canRetry = true,
          canResume = true,
        )

      else ->
        OnboardingState(
          surfaceId = surfaceId,
          state = OnboardingState.Status.SESSION_PENDING,
          trustMode = TrustMode.BRIDGE_LINKED,
          transportMode = TransportMode.CLI_BRIDGE,
          canRetry = true,
        )
    }
  }

  private fun blockedState(
    surfaceId: SurfaceId,
    recoveryKind: RecoveryKind,
    canRetry: Boolean = true,
    canResume: Boolean = false,
  ) =
    OnboardingState(
      surfaceId = surfaceId,
      state = OnboardingState.Status.BLOCKED,
      trustMode = TrustMode.BRIDGE_LINKED,
      transportMode = TransportMode.CLI_BRIDGE,
      recoveryKind = recoveryKind,
      canRetry = canRetry,
      canResume = canResume,
    )
}

data class MobileRuntimeCapabilities(
  val streamingResponses: Boolean = false,
  val resumableSessionList: Boolean = false,
  val approvalRequests: Boolean = false,
)

data class MobileRuntimeReadinessSnapshot(
  val runtimeAvailable: Boolean,
  val linkEstablished: Boolean,
  val sessionCapable: Boolean,
  val activeSessionId: SessionId? = null,
  val environmentSupported: Boolean = true,
  val capabilities: MobileRuntimeCapabilities = MobileRuntimeCapabilities(),
) {
  fun toOnboardingState(surfaceId: SurfaceId = SurfaceId.COMPOSEAPP_MOBILE): OnboardingState {
    require(surfaceId == SurfaceId.COMPOSEAPP_MOBILE) {
      "MobileRuntimeReadinessSnapshot only supports the ${SurfaceId.COMPOSEAPP_MOBILE} surface contract"
    }

    return when {
      !environmentSupported ->
        blockedState(
          surfaceId = surfaceId,
          recoveryKind = RecoveryKind.ENVIRONMENT_UNSUPPORTED,
          canRetry = false,
        )

      !runtimeAvailable ->
        blockedState(surfaceId = surfaceId, recoveryKind = RecoveryKind.RUNTIME_UNAVAILABLE)

      !linkEstablished ->
        OnboardingState(
          surfaceId = surfaceId,
          state = OnboardingState.Status.TRUST_PENDING,
          trustMode = TrustMode.BRIDGE_LINKED,
          transportMode = TransportMode.CLI_BRIDGE,
          canRetry = true,
        )

      !sessionCapable ->
        blockedState(
          surfaceId = surfaceId,
          recoveryKind = RecoveryKind.LINKED_BUT_NOT_SESSION_READY,
          canResume = activeSessionId != null,
        )

      activeSessionId != null ->
        OnboardingState(
          surfaceId = surfaceId,
          state = OnboardingState.Status.SESSION_READY,
          trustMode = TrustMode.BRIDGE_LINKED,
          transportMode = TransportMode.CLI_BRIDGE,
          canRetry = true,
          canResume = true,
        )

      else ->
        OnboardingState(
          surfaceId = surfaceId,
          state = OnboardingState.Status.SESSION_PENDING,
          trustMode = TrustMode.BRIDGE_LINKED,
          transportMode = TransportMode.CLI_BRIDGE,
          canRetry = true,
        )
    }
  }

  private fun blockedState(
    surfaceId: SurfaceId,
    recoveryKind: RecoveryKind,
    canRetry: Boolean = true,
    canResume: Boolean = false,
  ) =
    OnboardingState(
      surfaceId = surfaceId,
      state = OnboardingState.Status.BLOCKED,
      trustMode = TrustMode.BRIDGE_LINKED,
      transportMode = TransportMode.CLI_BRIDGE,
      recoveryKind = recoveryKind,
      canRetry = canRetry,
      canResume = canResume,
    )
}

data class MobileRuntimeSession(
  val id: SessionId,
  val title: String? = null,
  val isActive: Boolean = false,
)

data class MobileApprovalRequest(
  val id: String,
  val sessionId: SessionId,
  val toolLabel: String,
  val reason: String,
) {
  init {
    require(id.isNotBlank()) { "Approval request identifiers must not be blank." }
    require(toolLabel.isNotBlank()) { "Approval tool labels must not be blank." }
    require(reason.isNotBlank()) { "Approval reasons must not be blank." }
  }
}

enum class MobileApprovalDecision {
  APPROVE,
  DENY,
}

sealed interface MobileRuntimeEvent {
  data class AssistantChunk(val sessionId: SessionId, val text: String) : MobileRuntimeEvent

  data class AssistantMessage(val sessionId: SessionId, val text: String) : MobileRuntimeEvent

  data class ApprovalPending(val request: MobileApprovalRequest) : MobileRuntimeEvent

  data class Failure(val sessionId: SessionId, val message: String, val recoverable: Boolean) :
    MobileRuntimeEvent
}

data class MobileRuntimeTurnResult(val sessionId: SessionId, val events: List<MobileRuntimeEvent>)

data class LinkedRuntimeMetadata(
  val targetId: String,
  val transportMode: TransportMode = TransportMode.CLI_BRIDGE,
  val trustMode: TrustMode = TrustMode.BRIDGE_LINKED,
  val linkedAtEpochMs: Long? = null,
) {
  init {
    require(targetId.isNotBlank()) { "Runtime target identifiers must not be blank." }
  }
}

// Client-first connection model (Task 1.2)
data class RuntimeConnectionTarget(
  val id: String,
  val label: String,
  val method: RuntimeConnectionMethod,
  val endpointUrl: String? = null,
)

data class RuntimeTrustState(val established: Boolean, val requiresPairingOrAuth: Boolean)

data class RuntimeConnectionReadinessSnapshot(
  val target: RuntimeConnectionTarget?,
  val trustState: RuntimeTrustState,
  val transportReachable: Boolean,
  val sessionCapable: Boolean,
  val activeSessionId: SessionId? = null,
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

sealed interface CoreResult {
  data class Success(val output: CoreOutput) : CoreResult

  data class Failure(
    val message: String,
    val details: String? = null,
    val recoverable: Boolean = false,
  ) : CoreResult
}

fun interface AgentCoreBridge {
  fun invoke(invocation: CoreInvocation): CoreResult
}
