package com.profiletailors.corvus

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.tooling.preview.Preview
import com.profiletailors.corvus.ui.chat.ChatWorkspace
import com.profiletailors.corvus.ui.chat.ChatWorkspaceDefaults
import com.profiletailors.corvus.ui.chat.MobileBridgeSnapshot
import com.profiletailors.corvus.ui.chat.MobileOnboardingStatus
import com.profiletailors.corvus.ui.onboarding.OnboardingDefaults
import com.profiletailors.corvus.ui.onboarding.OnboardingScreen
import com.profiletailors.corvus.ui.theme.CorvusTheme
import kotlin.random.Random

private const val AGENT_NAME = "Corvus Agent"

@Composable
@Preview
fun App(platformOverride: Platform? = null, initialBridgeSnapshot: MobileBridgeSnapshot? = null) {
  val platform = remember(platformOverride) { platformOverride ?: getPlatform() }
  var onboardingStepIndex by rememberSaveable { mutableIntStateOf(0) }
  val onboardingSteps = remember { OnboardingDefaults.steps }
  var bridgeSnapshot by
    remember(platform, initialBridgeSnapshot) {
      mutableStateOf(initialBridgeSnapshot ?: defaultBridgeSnapshotFor(platform))
    }
  val shouldShowOnboarding =
    remember(platform, onboardingStepIndex, onboardingSteps) {
      platform.isMobile && onboardingStepIndex < onboardingSteps.size
    }

  CorvusTheme {
    if (shouldShowOnboarding) {
      OnboardingScreen(
        step = onboardingSteps[onboardingStepIndex],
        currentStepIndex = onboardingStepIndex,
        totalSteps = onboardingSteps.size,
        isLastStep = onboardingStepIndex == onboardingSteps.lastIndex,
        onSkip = { onboardingStepIndex = onboardingSteps.size },
        onNext = {
          onboardingStepIndex =
            if (onboardingStepIndex < onboardingSteps.lastIndex) {
              onboardingStepIndex + 1
            } else {
              onboardingSteps.size
            }
        },
      )
    } else {
      ChatWorkspace(
        state = ChatWorkspaceDefaults.state(modelName = AGENT_NAME),
        bridgeSnapshot = bridgeSnapshot,
        platformName = platform.name,
        onRetryBridge = {
          if (bridgeSnapshot.environmentSupported) {
            bridgeSnapshot = bridgeSnapshot.copy(runtimeAvailable = true, sessionCapable = true)
          }
        },
        onLinkSurface = {
          if (bridgeSnapshot.environmentSupported) {
            bridgeSnapshot =
              bridgeSnapshot.copy(
                runtimeAvailable = true,
                linkEstablished = true,
                sessionCapable = true,
              )
          }
        },
        onStartSession = {
          if (bridgeSnapshot.toOnboardingState().status == MobileOnboardingStatus.SESSION_PENDING) {
            bridgeSnapshot = bridgeSnapshot.copy(sessionId = generateSessionId())
          }
        },
        onClearSession = {
          if (bridgeSnapshot.environmentSupported) {
            bridgeSnapshot =
              bridgeSnapshot.copy(linkEstablished = false, sessionCapable = false, sessionId = null)
          }
        },
      )
    }
  }
}

internal fun defaultBridgeSnapshotFor(platform: Platform): MobileBridgeSnapshot =
  when {
    !platform.isMobile ->
      MobileBridgeSnapshot(
        runtimeAvailable = true,
        linkEstablished = true,
        sessionCapable = true,
        sessionId = "desktop-preview-session",
      )

    platform.bridgeAvailability == BridgeAvailability.LOCAL_BRIDGE ->
      MobileBridgeSnapshot(runtimeAvailable = true, linkEstablished = false, sessionCapable = true)

    else ->
      MobileBridgeSnapshot(
        runtimeAvailable = false,
        linkEstablished = false,
        sessionCapable = false,
        environmentSupported = false,
      )
  }

private fun generateSessionId(): String =
  listOf(8, 4, 4, 4, 12).joinToString("-") { segmentLength ->
    buildString(segmentLength) {
      repeat(segmentLength) { append(HEX_DIGITS[Random.nextInt(HEX_DIGITS.length)]) }
    }
  }

private const val HEX_DIGITS = "0123456789abcdef"
