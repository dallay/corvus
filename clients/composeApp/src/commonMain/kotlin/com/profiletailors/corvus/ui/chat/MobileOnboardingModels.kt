package com.profiletailors.corvus.ui.chat

import com.profiletailors.corvus.runtime.RuntimeConnectionMethod
import com.profiletailors.corvus.runtime.toCoreOnboardingState
import com.profiletailors.corvus.runtime.toMobileOnboardingState

// Client-first trust modes (Task 1.2)
enum class MobileTrustMode {
  BRIDGE_LINKED,
  PAIRING_REQUIRED,
  TRUSTED_COMPANION_ESTABLISHED,
}

// Client-first transport modes (Task 1.2)
enum class MobileTransportMode {
  CLI_BRIDGE,
  ENDPOINT_URL,
  TRUSTED_COMPANION,
  LOCAL_HOST_ADVANCED,
}

// Client-first recovery kinds (Task 1.2)
enum class MobileRecoveryKind {
  RUNTIME_UNAVAILABLE,
  TRANSPORT_UNAVAILABLE,
  LINKED_BUT_NOT_SESSION_READY,
  SESSION_UNAVAILABLE,
  ENVIRONMENT_UNSUPPORTED,
  // Client-first recovery kinds
  NO_TARGET_CONFIGURED,
  TARGET_NOT_REACHABLE,
  TRUST_NOT_ESTABLISHED,
}

// Client-first onboarding statuses (Task 1.2)
enum class MobileOnboardingStatus {
  TARGET_SELECTED, // User has selected a connection target
  RECOVERY, // Recovery action needed
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
  // Client-first fields (Task 1.2)
  val connectionMethod: RuntimeConnectionMethod? = null,
  val targetId: String? = null,
)

data class MobileBridgeSnapshot(
  val runtimeAvailable: Boolean,
  val linkEstablished: Boolean,
  val sessionCapable: Boolean,
  val sessionId: String? = null,
  val environmentSupported: Boolean = true,
  val recoveryOverride: MobileRecoveryKind? = null,
  val targetLabel: String? = null,
  // Client-first fields (Task 1.2)
  val connectionMethod: RuntimeConnectionMethod? = null,
  val targetId: String? = null,
) {
  fun toOnboardingState(): MobileOnboardingState = toCoreOnboardingState().toMobileOnboardingState()
}
