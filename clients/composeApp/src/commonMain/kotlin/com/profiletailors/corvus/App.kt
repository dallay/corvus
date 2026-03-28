@file:Suppress("FunctionNaming")

package com.profiletailors.corvus

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.tooling.preview.Preview
import com.profiletailors.corvus.ui.chat.BridgeActions
import com.profiletailors.corvus.ui.chat.ChatWorkspace
import com.profiletailors.corvus.ui.chat.ChatWorkspaceDefaults
import com.profiletailors.corvus.ui.chat.MobileBridgeSnapshot
import com.profiletailors.corvus.ui.chat.MobileOnboardingStatus
import com.profiletailors.corvus.ui.onboarding.OnboardingDefaults
import com.profiletailors.corvus.ui.onboarding.OnboardingScreen
import com.profiletailors.corvus.ui.onboarding.OnboardingStep
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
      AppOnboardingContent(
        steps = onboardingSteps,
        stepIndex = onboardingStepIndex,
        onStepIndexChange = { onboardingStepIndex = it },
      )
    } else {
      AppChatContent(
        platform = platform,
        bridgeSnapshot = bridgeSnapshot,
        onBridgeSnapshotChange = { bridgeSnapshot = it },
      )
    }
  }
}

@Composable
private fun AppOnboardingContent(
  steps: List<OnboardingStep>,
  stepIndex: Int,
  onStepIndexChange: (Int) -> Unit,
) {
  OnboardingScreen(
    step = steps[stepIndex],
    currentStepIndex = stepIndex,
    totalSteps = steps.size,
    isLastStep = stepIndex == steps.lastIndex,
    onSkip = { onStepIndexChange(steps.size) },
    onNext = {
      val next = if (stepIndex < steps.lastIndex) stepIndex + 1 else steps.size
      onStepIndexChange(next)
    },
  )
}

@Composable
private fun AppChatContent(
  platform: Platform,
  bridgeSnapshot: MobileBridgeSnapshot,
  onBridgeSnapshotChange: (MobileBridgeSnapshot) -> Unit,
) {
  val currentSnapshot by rememberUpdatedState(bridgeSnapshot)
  val currentOnChange by rememberUpdatedState(onBridgeSnapshotChange)
  val bridgeActions = remember {
    BridgeActions(
      onRetryBridge = {
        if (currentSnapshot.environmentSupported) {
          currentOnChange(currentSnapshot.copy(runtimeAvailable = true, sessionCapable = true))
        }
      },
      onLinkSurface = {
        if (currentSnapshot.environmentSupported) {
          currentOnChange(
            currentSnapshot.copy(
              runtimeAvailable = true,
              linkEstablished = true,
              sessionCapable = true,
            )
          )
        }
      },
      onStartSession = {
        if (currentSnapshot.toOnboardingState().status == MobileOnboardingStatus.SESSION_PENDING) {
          currentOnChange(currentSnapshot.copy(sessionId = generateSessionId()))
        }
      },
      onClearSession = {
        if (currentSnapshot.environmentSupported) {
          currentOnChange(
            currentSnapshot.copy(linkEstablished = false, sessionCapable = false, sessionId = null)
          )
        }
      },
    )
  }

  ChatWorkspace(
    state = ChatWorkspaceDefaults.state(modelName = AGENT_NAME),
    bridgeSnapshot = bridgeSnapshot,
    platformName = platform.name,
    bridgeActions = bridgeActions,
  )
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

private val UUID_SEGMENT_LENGTHS = listOf(8, 4, 4, 4, 12)

private fun generateSessionId(): String =
  UUID_SEGMENT_LENGTHS.joinToString("-") { segmentLength ->
    buildString(segmentLength) {
      repeat(segmentLength) { append(HEX_DIGITS[Random.nextInt(HEX_DIGITS.length)]) }
    }
  }

private const val HEX_DIGITS = "0123456789abcdef"
