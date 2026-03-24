package com.profiletailors.agent.core

data class CoreInvocation(
  val prompt: String,
  val sessionId: String? = null,
  val metadata: Map<String, String> = emptyMap(),
  val timeoutMs: Long? = null,
)

data class CoreOutput(val text: String, val transport: String, val rawOutput: String? = null)

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
