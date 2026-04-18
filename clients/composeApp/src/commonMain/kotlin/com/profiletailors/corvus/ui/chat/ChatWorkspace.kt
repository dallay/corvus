@file:Suppress("FunctionNaming")

package com.profiletailors.corvus.ui.chat

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeContentPadding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.Stable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
import com.profiletailors.corvus.runtime.RuntimeApprovalRequest
import com.profiletailors.corvus.runtime.RuntimeSession
import com.profiletailors.corvus.ui.theme.CorvusColorPalette
import com.profiletailors.corvus.ui.theme.CorvusTheme

@Immutable
data class ChatWorkspaceState(
  val modelName: String,
  val inputPlaceholder: String,
  val welcomeMessage: String,
)

@Immutable
data class MobileBridgeUiState(val platformName: String, val snapshot: MobileBridgeSnapshot) {
  val onboardingState: MobileOnboardingState = snapshot.toOnboardingState()
  val isChatReady: Boolean = onboardingState.status == MobileOnboardingStatus.SESSION_READY
  val onboardingStateLabel: String = mobileOnboardingStateLabel(onboardingState.status)
  val onboardingRecoveryLabel: String? =
    onboardingState.recoveryKind?.let(::mobileOnboardingRecoveryLabel)
}

fun mobileOnboardingStateLabel(status: MobileOnboardingStatus): String =
  when (status) {
    MobileOnboardingStatus.TARGET_SELECTED -> "target_selected"
    MobileOnboardingStatus.RECOVERY -> "recovery"
    MobileOnboardingStatus.RUNTIME_PATH_CONFIRMED -> "runtime_path_confirmed"
    MobileOnboardingStatus.TRUST_PENDING -> "trust_pending"
    MobileOnboardingStatus.TRANSPORT_CONNECTING -> "transport_connecting"
    MobileOnboardingStatus.SESSION_PENDING -> "session_pending"
    MobileOnboardingStatus.SESSION_READY -> "session_ready"
    MobileOnboardingStatus.BLOCKED -> "blocked"
  }

fun mobileOnboardingTransitionLabel(
  from: MobileOnboardingStatus,
  to: MobileOnboardingStatus,
): String = "${mobileOnboardingStateLabel(from)}__to__${mobileOnboardingStateLabel(to)}"

fun mobileOnboardingRecoveryLabel(recoveryKind: MobileRecoveryKind): String =
  when (recoveryKind) {
    MobileRecoveryKind.NO_TARGET_CONFIGURED -> "no_target_configured"
    MobileRecoveryKind.TARGET_NOT_REACHABLE -> "target_not_reachable"
    MobileRecoveryKind.TRUST_NOT_ESTABLISHED -> "trust_not_established"
    MobileRecoveryKind.RUNTIME_UNAVAILABLE -> "runtime_unavailable"
    MobileRecoveryKind.TRANSPORT_UNAVAILABLE -> "transport_unavailable"
    MobileRecoveryKind.LINKED_BUT_NOT_SESSION_READY -> "linked_but_not_session_ready"
    MobileRecoveryKind.SESSION_UNAVAILABLE -> "session_unavailable"
    MobileRecoveryKind.ENVIRONMENT_UNSUPPORTED -> "environment_unsupported"
  }

@Stable
data class ChatUiState(
  val workspaceState: ChatWorkspaceState,
  val bridgeState: MobileBridgeUiState,
  val messages: List<ChatMessage>,
  val resumableSessions: List<RuntimeSession>,
  val pendingApproval: RuntimeApprovalRequest?,
  val targetLabel: String?,
  val activeSessionId: String?,
  val query: String,
  val showConfig: Boolean,
)

@Stable
data class BridgeActions(
  val onRetryBridge: () -> Unit,
  val onLinkSurface: () -> Unit,
  val onStartSession: () -> Unit,
  val onResumeSession: (String) -> Unit,
  val onDisconnectReset: () -> Unit,
  val onApprove: () -> Unit,
  val onDeny: () -> Unit,
)

@Stable
data class ChatWorkspaceActions(
  val onQueryChange: (String) -> Unit,
  val onSend: (String) -> Unit,
  val onToggleConfig: () -> Unit,
  val bridge: BridgeActions,
)

@Stable
data class ChatWorkspaceContent(
  val bridgeSnapshot: MobileBridgeSnapshot,
  val platformName: String,
  val messages: List<ChatMessage>,
  val resumableSessions: List<RuntimeSession>,
  val pendingApproval: RuntimeApprovalRequest?,
  val targetLabel: String?,
  val activeSessionId: String?,
  val query: String,
  val showConfig: Boolean,
)

object ChatWorkspaceDefaults {
  const val DefaultAgentName = "Corvus"

  fun state(modelName: String = DefaultAgentName): ChatWorkspaceState =
    ChatWorkspaceState(
      modelName = modelName,
      inputPlaceholder = "Message your linked Corvus session...",
      welcomeMessage =
        "Corvus is ready to chat once this mobile surface is linked and a session is active.",
    )
}

@Composable
fun ChatWorkspace(
  content: ChatWorkspaceContent,
  bridgeActions: BridgeActions,
  onSendMessage: (String) -> Unit,
  onQueryChange: (String) -> Unit,
  onShowConfigChange: (Boolean) -> Unit,
  modifier: Modifier = Modifier,
  state: ChatWorkspaceState = ChatWorkspaceDefaults.state(),
) {

  val bridgeState =
    remember(content.platformName, content.bridgeSnapshot) {
      MobileBridgeUiState(platformName = content.platformName, snapshot = content.bridgeSnapshot)
    }

  fun sendMessage(prompt: String) {
    if (!bridgeState.isChatReady) return
    if (prompt.trim().isBlank()) return
    onSendMessage(prompt)
    onQueryChange("")
  }

  val displayMessages =
    remember(content.messages, state.welcomeMessage) {
      if (content.messages.isEmpty()) {
        listOf(ChatMessage(id = 0, role = ChatRole.Assistant, content = state.welcomeMessage))
      } else {
        content.messages
      }
    }

  val actions =
    remember(bridgeState, bridgeActions, onSendMessage, onQueryChange, content.showConfig, onShowConfigChange) {
      ChatWorkspaceActions(
        onQueryChange = onQueryChange,
        onSend = ::sendMessage,
        onToggleConfig = { onShowConfigChange(!content.showConfig) },
        bridge = bridgeActions,
      )
    }

  val uiState =
    ChatUiState(
      workspaceState = state,
      bridgeState = bridgeState,
      messages = displayMessages,
      resumableSessions = content.resumableSessions,
      pendingApproval = content.pendingApproval,
      targetLabel = content.targetLabel,
      activeSessionId = content.activeSessionId,
      query = content.query,
      showConfig = content.showConfig,
    )

  ChatWorkspaceScreen(uiState = uiState, actions = actions, modifier = modifier)
}

@Composable
private fun ChatWorkspaceScreen(
  uiState: ChatUiState,
  actions: ChatWorkspaceActions,
  modifier: Modifier = Modifier,
) {
  val colors = MaterialTheme.colorScheme
  val corvusColors = CorvusTheme.colors
  val shouldShowConfig = uiState.showConfig || !uiState.bridgeState.isChatReady

  val staticModifier =
    remember {
      Modifier.fillMaxSize()
        .safeContentPadding()
        .padding(horizontal = 20.dp, vertical = 16.dp)
    }
  val backgroundModifier =
    remember(colors.background) {
      Modifier.background(colors.background)
    }

  Column(modifier = modifier.then(backgroundModifier).then(staticModifier)) {
    ChatHeader(
      modelName = uiState.workspaceState.modelName,
      bridgeState = uiState.bridgeState,
      showConfig = shouldShowConfig,
      onToggleConfig = actions.onToggleConfig,
    )

    Spacer(modifier = Modifier.height(16.dp))

    WorkspaceDivider(corvusColors = corvusColors)

    Spacer(modifier = Modifier.height(16.dp))

    WorkspaceBody(
      uiState = uiState,
      actions = actions,
      shouldShowConfig = shouldShowConfig,
      modifier = Modifier.weight(1f),
    )
  }
}

@Composable
private fun WorkspaceDivider(corvusColors: CorvusColorPalette) {
  val dividerBrush =
    remember(corvusColors.glowPurple, corvusColors.glowCyan) {
      Brush.horizontalGradient(
        listOf(
          Color.Transparent,
          corvusColors.glowPurple.copy(alpha = 0.5f),
          corvusColors.glowCyan.copy(alpha = 0.5f),
          Color.Transparent,
        )
      )
    }

  Box(modifier = Modifier.fillMaxWidth().height(1.dp).background(brush = dividerBrush))
}

@Composable
private fun WorkspaceBody(
  uiState: ChatUiState,
  actions: ChatWorkspaceActions,
  shouldShowConfig: Boolean,
  modifier: Modifier = Modifier,
) {
  if (shouldShowConfig) {
    ConfigPanel(
      bridgeState = uiState.bridgeState,
      resumableSessions = uiState.resumableSessions,
      activeSessionId = uiState.activeSessionId,
      targetLabel = uiState.targetLabel,
      actions = actions,
      modifier = modifier,
    )
    return
  }

  val panelState =
    remember(uiState.workspaceState, uiState.bridgeState, uiState.pendingApproval) {
      ChatPanelState(
        workspaceState = uiState.workspaceState,
        bridgeState = uiState.bridgeState,
        pendingApproval = uiState.pendingApproval,
      )
    }
  ChatPanel(
    panelState = panelState,
    messages = uiState.messages,
    query = uiState.query,
    actions = actions,
    modifier = modifier,
  )
}

@Immutable
private data class ChatPanelState(
  val workspaceState: ChatWorkspaceState,
  val bridgeState: MobileBridgeUiState,
  val pendingApproval: RuntimeApprovalRequest?,
)

@Composable
private fun ChatPanel(
  panelState: ChatPanelState,
  messages: List<ChatMessage>,
  query: String,
  actions: ChatWorkspaceActions,
  modifier: Modifier = Modifier,
) {
  Column(modifier = modifier) {
    BridgeStatusCard(bridgeState = panelState.bridgeState, modifier = Modifier.fillMaxWidth())

    Spacer(modifier = Modifier.height(16.dp))

    MessageList(
      messages = messages,
      modelName = panelState.workspaceState.modelName,
      modifier = Modifier.fillMaxWidth().weight(1f),
    )

    Spacer(modifier = Modifier.height(16.dp))

    panelState.pendingApproval?.let { request ->
      ApprovalCard(
        request = request,
        onApprove = actions.bridge.onApprove,
        onDeny = actions.bridge.onDeny,
        modifier = Modifier.fillMaxWidth(),
      )
      Spacer(modifier = Modifier.height(16.dp))
    }

    val inputProps =
      remember(query, actions.onQueryChange, actions.onSend, panelState) {
        ChatInputFieldProps(
          value = query,
          onValueChange = actions.onQueryChange,
          onSend = actions.onSend,
          placeholder = panelState.workspaceState.inputPlaceholder,
          enabled = panelState.bridgeState.isChatReady,
        )
      }
    ChatInputField(props = inputProps)
  }
}

@Composable
private fun MessageList(
  messages: List<ChatMessage>,
  modelName: String,
  modifier: Modifier = Modifier,
) {
  val corvusColors = CorvusTheme.colors

  Surface(
    modifier = modifier,
    shape = RoundedCornerShape(20.dp),
    color = corvusColors.glassSurface,
  ) {
    val backgroundBrush = remember {
      Brush.verticalGradient(listOf(Color.White.copy(alpha = 0.05f), Color.Transparent))
    }

    Box(modifier = Modifier.background(brush = backgroundBrush)) {
      LazyColumn(
        modifier = Modifier.fillMaxSize().padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(16.dp),
      ) {
        items(items = messages, key = { it.id }, contentType = { it.role }) { message ->
          ChatBubble(message = message, modelName = modelName)
        }
      }
    }
  }
}
