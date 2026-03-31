@file:Suppress("FunctionNaming")

package com.profiletailors.corvus

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.tooling.preview.Preview
import com.profiletailors.corvus.runtime.MobileRuntimeCoordinator
import com.profiletailors.corvus.runtime.RuntimeApprovalDecision
import com.profiletailors.corvus.runtime.RuntimeSessionId
import com.profiletailors.corvus.runtime.rememberPlatformRuntimeDependencies
import com.profiletailors.corvus.ui.chat.ChatWorkspace
import com.profiletailors.corvus.ui.chat.ChatWorkspaceDefaults
import com.profiletailors.corvus.ui.chat.MobileBridgeSnapshot
import com.profiletailors.corvus.ui.chat.MobileOnboardingState
import com.profiletailors.corvus.ui.chat.MobileOnboardingStatus
import com.profiletailors.corvus.ui.onboarding.OnboardingScreen
import com.profiletailors.corvus.ui.onboarding.runtimeOnboardingStep
import com.profiletailors.corvus.ui.theme.CorvusTheme
import java.util.UUID

private const val AGENT_NAME = "Corvus Agent"

private data class ChatBindings(
  val coordinatorState: com.profiletailors.corvus.runtime.MobileRuntimeCoordinatorState,
  val onRetryBridge: () -> Unit,
  val onLinkSurface: () -> Unit,
  val onStartSession: () -> Unit,
  val onResumeSession: (String) -> Unit,
  val onSendMessage: (String) -> Unit,
  val onDisconnectReset: () -> Unit,
  val onApprove: () -> Unit,
  val onDeny: () -> Unit,
)

@Composable
@Preview
fun App(platformOverride: Platform? = null, initialBridgeSnapshot: MobileBridgeSnapshot? = null) {
  val platform = remember(platformOverride) { platformOverride ?: getPlatform() }
  val dependencies =
    rememberPlatformRuntimeDependencies(initialBridgeSnapshot = initialBridgeSnapshot)
  val facade = dependencies.facade
  val persistence = dependencies.persistence
  val coordinator = remember(facade, persistence) { MobileRuntimeCoordinator(facade, persistence) }
  var coordinatorState by remember(coordinator) { mutableStateOf(coordinator.state) }

  LaunchedEffect(coordinator) {
    coordinator.refresh()
    coordinatorState = coordinator.state
  }

  fun mutateCoordinator(block: MobileRuntimeCoordinator.() -> Unit) {
    coordinator.block()
    coordinatorState = coordinator.state
  }

  val onboardingState = coordinatorState.bridgeSnapshot.toOnboardingState()
  // Client-first: show onboarding when not in SESSION_READY (regardless of platform)
  val shouldShowOnboarding = onboardingState.status != MobileOnboardingStatus.SESSION_READY

  CorvusTheme {
    if (shouldShowOnboarding) {
      AppOnboardingContent(
        onboardingState = onboardingState,
        onSkip = { mutateCoordinator { refresh() } },
        onPrimaryAction = {
          when (onboardingState.status) {
            // Client-first onboarding states
            MobileOnboardingStatus.TARGET_SELECTED,
            MobileOnboardingStatus.RECOVERY,
            MobileOnboardingStatus.TRUST_PENDING -> {
              mutateCoordinator { refresh() }
            }
            MobileOnboardingStatus.SESSION_PENDING -> mutateCoordinator { startNewSession() }
            MobileOnboardingStatus.BLOCKED -> mutateCoordinator { refresh() }
            else -> mutateCoordinator { refresh() }
          }
        },
      )
    } else {
      AppChatContent(
        platform = platform,
        bindings =
          ChatBindings(
            coordinatorState = coordinatorState,
            onRetryBridge = { mutateCoordinator { refresh() } },
            onLinkSurface = { mutateCoordinator { refresh() } },
            onStartSession = { mutateCoordinator { startNewSession() } },
            onResumeSession = { sessionId ->
              mutateCoordinator { resumeSession(RuntimeSessionId(sessionId)) }
            },
            onSendMessage = { prompt -> mutateCoordinator { sendMessage(prompt) } },
            onDisconnectReset = { mutateCoordinator { disconnect() } },
            onApprove = { mutateCoordinator { submitApproval(RuntimeApprovalDecision.APPROVE) } },
            onDeny = { mutateCoordinator { submitApproval(RuntimeApprovalDecision.DENY) } },
          ),
      )
    }
  }
}

@Composable
private fun AppOnboardingContent(
  onboardingState: MobileOnboardingState,
  onSkip: () -> Unit,
  onPrimaryAction: () -> Unit,
) {
  val step = runtimeOnboardingStep(onboardingState)
  OnboardingScreen(
    step = step,
    currentStepIndex = step.progressIndex,
    totalSteps = step.totalSteps,
    isLastStep = step.isTerminal,
    primaryActionLabel = step.actionLabel,
    onSkip = onSkip,
    onNext = onPrimaryAction,
  )
}

@Composable
private fun AppChatContent(platform: Platform, bindings: ChatBindings) {
  ChatWorkspace(
    state = ChatWorkspaceDefaults.state(modelName = AGENT_NAME),
    bridgeSnapshot = bindings.coordinatorState.bridgeSnapshot,
    platformName = platform.name,
    messages = bindings.coordinatorState.messages,
    pendingApproval = bindings.coordinatorState.pendingApproval,
    resumableSessions = bindings.coordinatorState.resumableSessions,
    targetLabel = bindings.coordinatorState.targetLabel,
    activeSessionId = bindings.coordinatorState.activeSessionId?.value,
    onRetryBridge = bindings.onRetryBridge,
    onLinkSurface = bindings.onLinkSurface,
    onStartSession = bindings.onStartSession,
    onResumeSession = bindings.onResumeSession,
    onSendMessage = bindings.onSendMessage,
    onDisconnectReset = bindings.onDisconnectReset,
    onApprove = bindings.onApprove,
    onDeny = bindings.onDeny,
  )
}

internal fun launchBridgeSnapshotFor(
  platform: Platform,
  preview: Boolean = false,
): MobileBridgeSnapshot? = if (preview) defaultPreviewBridgeSnapshotFor(platform) else null

internal fun defaultPreviewBridgeSnapshotFor(platform: Platform): MobileBridgeSnapshot =
  when {
    !platform.isMobile ->
      MobileBridgeSnapshot(
        runtimeAvailable = true,
        linkEstablished = true,
        sessionCapable = true,
        sessionId = UUID.randomUUID().toString(),
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
