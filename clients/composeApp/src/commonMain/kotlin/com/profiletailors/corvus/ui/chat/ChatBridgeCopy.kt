package com.profiletailors.corvus.ui.chat

internal const val RECOVERY_REQUIRED_LABEL = "Recovery required"
internal const val TARGET_SELECTED_LABEL = "Target selected"

internal fun onboardingStateLabel(onboardingState: MobileOnboardingState): String =
  when (onboardingState.status) {
    MobileOnboardingStatus.TARGET_SELECTED -> TARGET_SELECTED_LABEL
    MobileOnboardingStatus.RECOVERY -> RECOVERY_REQUIRED_LABEL
    MobileOnboardingStatus.RUNTIME_PATH_CONFIRMED -> "Runtime available"
    MobileOnboardingStatus.TRUST_PENDING -> "Trust this surface"
    MobileOnboardingStatus.TRANSPORT_CONNECTING -> "Connect to runtime"
    MobileOnboardingStatus.SESSION_PENDING -> "Session pending"
    MobileOnboardingStatus.SESSION_READY -> "Session ready"
    MobileOnboardingStatus.BLOCKED -> RECOVERY_REQUIRED_LABEL
  }

internal fun bridgeStateHeadline(bridgeState: MobileBridgeUiState): String =
  when (bridgeState.onboardingState.status) {
    MobileOnboardingStatus.TARGET_SELECTED -> TARGET_SELECTED_LABEL
    MobileOnboardingStatus.RECOVERY -> RECOVERY_REQUIRED_LABEL
    MobileOnboardingStatus.TRUST_PENDING -> "Connect to runtime endpoint"
    MobileOnboardingStatus.SESSION_PENDING -> "Runtime connected. Start or resume a session next."
    MobileOnboardingStatus.SESSION_READY -> "Session ready on ${bridgeState.platformName}"
    MobileOnboardingStatus.BLOCKED -> "Connection attention needed on ${bridgeState.platformName}"
    MobileOnboardingStatus.RUNTIME_PATH_CONFIRMED -> "Runtime endpoint configured"
    MobileOnboardingStatus.TRANSPORT_CONNECTING -> "Connecting to runtime"
  }

internal fun bridgeStateDescription(bridgeState: MobileBridgeUiState): String =
  when (bridgeState.onboardingState.status) {
    MobileOnboardingStatus.TRUST_PENDING ->
      "Configure connection to an existing runtime before starting a session."
    MobileOnboardingStatus.SESSION_PENDING ->
      "The runtime endpoint is reachable. Create or resume a chat session to continue."
    MobileOnboardingStatus.SESSION_READY ->
      "Messages now flow through the configured runtime connection."
    MobileOnboardingStatus.BLOCKED -> bridgeStateRecovery(bridgeState)
    else -> "Corvus keeps connection, transport, and session steps separate."
  }

internal fun bridgeStateRecovery(bridgeState: MobileBridgeUiState): String =
  when (bridgeState.onboardingState.recoveryKind) {
    MobileRecoveryKind.NO_TARGET_CONFIGURED ->
      "Select a connection target first. Choose endpoint URL, trusted companion, or another supported method."
    MobileRecoveryKind.TARGET_NOT_REACHABLE ->
      "The selected target is not reachable. Check your network connection and try again."
    MobileRecoveryKind.TRUST_NOT_ESTABLISHED ->
      "Trust has not been established. Complete the pairing or companion authentication first."
    MobileRecoveryKind.RUNTIME_UNAVAILABLE ->
      "Make sure the runtime is running and reachable, then retry."
    MobileRecoveryKind.TRANSPORT_UNAVAILABLE ->
      "The connection transport is unavailable. Retry first, or reconfigure the connection target."
    MobileRecoveryKind.LINKED_BUT_NOT_SESSION_READY ->
      "The runtime is connected, but session operations are unavailable. Retry the connection check."
    MobileRecoveryKind.SESSION_UNAVAILABLE ->
      "Your last session is no longer resumable. Start a new session or choose another available session."
    MobileRecoveryKind.ENVIRONMENT_UNSUPPORTED ->
      "This environment requires a supported connection method. Check your platform's available options."
    null -> fallbackRecovery(bridgeState.onboardingState.status)
  }

private fun fallbackRecovery(status: MobileOnboardingStatus): String =
  when (status) {
    MobileOnboardingStatus.TARGET_SELECTED ->
      "A target is selected but not yet configured. Complete the connection setup."
    MobileOnboardingStatus.RECOVERY -> "Recovery is needed. Follow the on-screen instructions."
    MobileOnboardingStatus.TRUST_PENDING ->
      "Configure the connection to your runtime endpoint first."
    MobileOnboardingStatus.SESSION_PENDING ->
      "Start a session after the runtime connection is confirmed reachable."
    else -> "No recovery action needed."
  }
