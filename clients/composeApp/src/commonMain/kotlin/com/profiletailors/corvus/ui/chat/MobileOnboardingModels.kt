package com.profiletailors.corvus.ui.chat

enum class MobileTrustMode {
  BRIDGE_LINKED
}

enum class MobileTransportMode {
  CLI_BRIDGE
}

enum class MobileRecoveryKind {
  RUNTIME_UNAVAILABLE,
  LINKED_BUT_NOT_SESSION_READY,
  ENVIRONMENT_UNSUPPORTED,
}

enum class MobileOnboardingStatus {
  RUNTIME_PATH_CONFIRMED,
  TRUST_PENDING,
  TRANSPORT_CONNECTING,
  SESSION_PENDING,
  SESSION_READY,
  BLOCKED,
}

data class MobileOnboardingState(
  val status: MobileOnboardingStatus,
  val trustMode: MobileTrustMode = MobileTrustMode.BRIDGE_LINKED,
  val transportMode: MobileTransportMode = MobileTransportMode.CLI_BRIDGE,
  val recoveryKind: MobileRecoveryKind? = null,
  val canRetry: Boolean = false,
  val canResume: Boolean = false,
)

data class MobileBridgeSnapshot(
  val runtimeAvailable: Boolean,
  val linkEstablished: Boolean,
  val sessionCapable: Boolean,
  val sessionId: String? = null,
  val environmentSupported: Boolean = true,
) {
  fun toOnboardingState(): MobileOnboardingState =
    when {
      !environmentSupported ->
        MobileOnboardingState(
          status = MobileOnboardingStatus.BLOCKED,
          recoveryKind = MobileRecoveryKind.ENVIRONMENT_UNSUPPORTED,
        )

      !runtimeAvailable ->
        MobileOnboardingState(
          status = MobileOnboardingStatus.BLOCKED,
          recoveryKind = MobileRecoveryKind.RUNTIME_UNAVAILABLE,
          canRetry = true,
        )

      !linkEstablished ->
        MobileOnboardingState(status = MobileOnboardingStatus.TRUST_PENDING, canRetry = true)

      !sessionCapable ->
        MobileOnboardingState(
          status = MobileOnboardingStatus.BLOCKED,
          recoveryKind = MobileRecoveryKind.LINKED_BUT_NOT_SESSION_READY,
          canRetry = true,
          canResume = !sessionId.isNullOrBlank(),
        )

      !sessionId.isNullOrBlank() ->
        MobileOnboardingState(
          status = MobileOnboardingStatus.SESSION_READY,
          canRetry = true,
          canResume = true,
        )

      else ->
        MobileOnboardingState(status = MobileOnboardingStatus.SESSION_PENDING, canRetry = true)
    }
}
