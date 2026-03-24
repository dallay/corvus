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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.unit.dp
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
    MobileRecoveryKind.RUNTIME_UNAVAILABLE -> "runtime_unavailable"
    MobileRecoveryKind.LINKED_BUT_NOT_SESSION_READY -> "linked_but_not_session_ready"
    MobileRecoveryKind.ENVIRONMENT_UNSUPPORTED -> "environment_unsupported"
  }

@Stable
data class ChatUiState(
  val workspaceState: ChatWorkspaceState,
  val bridgeState: MobileBridgeUiState,
  val messages: List<ChatMessage>,
  val query: String,
  val showConfig: Boolean,
)

@Stable
data class ChatWorkspaceActions(
  val onQueryChange: (String) -> Unit,
  val onSend: () -> Unit,
  val onToggleConfig: () -> Unit,
  val onRetryBridge: () -> Unit,
  val onLinkSurface: () -> Unit,
  val onStartSession: () -> Unit,
  val onClearSession: () -> Unit,
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
  bridgeSnapshot: MobileBridgeSnapshot,
  platformName: String,
  onRetryBridge: () -> Unit,
  onLinkSurface: () -> Unit,
  onStartSession: () -> Unit,
  onClearSession: () -> Unit,
  modifier: Modifier = Modifier,
  state: ChatWorkspaceState = ChatWorkspaceDefaults.state(),
) {
  var query by remember { mutableStateOf("") }
  var nextId by rememberSaveable { mutableIntStateOf(1) }
  var showConfig by rememberSaveable { mutableStateOf(false) }

  val messages =
    remember(state.welcomeMessage) {
      mutableStateListOf(
        ChatMessage(id = 0, role = ChatRole.Assistant, content = state.welcomeMessage)
      )
    }

  val bridgeState =
    remember(platformName, bridgeSnapshot) {
      MobileBridgeUiState(platformName = platformName, snapshot = bridgeSnapshot)
    }

  fun sendMessage() {
    if (!bridgeState.isChatReady) return

    val prompt = query.trim()
    if (prompt.isBlank()) return

    messages.add(ChatMessage(id = nextId, role = ChatRole.User, content = prompt))
    nextId += 1

    messages.add(
      ChatMessage(
        id = nextId,
        role = ChatRole.Assistant,
        content = buildLocalAssistantReply(prompt, state.modelName, bridgeState),
      )
    )
    nextId += 1
    query = ""
  }

  val actions =
    remember(onRetryBridge, onLinkSurface, onStartSession, onClearSession) {
      ChatWorkspaceActions(
        onQueryChange = { query = it },
        onSend = ::sendMessage,
        onToggleConfig = { showConfig = !showConfig },
        onRetryBridge = onRetryBridge,
        onLinkSurface = onLinkSurface,
        onStartSession = onStartSession,
        onClearSession = onClearSession,
      )
    }

  val uiState =
    remember(state, bridgeState, query, showConfig, messages) {
      ChatUiState(
        workspaceState = state,
        bridgeState = bridgeState,
        messages = messages,
        query = query,
        showConfig = showConfig,
      )
    }

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

  Column(
    modifier =
      modifier
        .fillMaxSize()
        .background(colors.background)
        .safeContentPadding()
        .padding(horizontal = 20.dp, vertical = 16.dp)
  ) {
    ChatHeader(
      modelName = uiState.workspaceState.modelName,
      bridgeState = uiState.bridgeState,
      showConfig = shouldShowConfig,
      onToggleConfig = actions.onToggleConfig,
    )

    Spacer(modifier = Modifier.height(16.dp))

    Box(
      modifier =
        Modifier.fillMaxWidth()
          .height(1.dp)
          .background(
            brush =
              Brush.horizontalGradient(
                listOf(
                  Color.Transparent,
                  corvusColors.glowPurple.copy(alpha = 0.5f),
                  corvusColors.glowCyan.copy(alpha = 0.5f),
                  Color.Transparent,
                )
              )
          )
    )

    Spacer(modifier = Modifier.height(16.dp))

    if (shouldShowConfig) {
      ConfigPanel(
        bridgeState = uiState.bridgeState,
        actions = actions,
        modifier = Modifier.weight(1f),
      )
    } else {
      ChatPanel(
        state = uiState.workspaceState,
        bridgeState = uiState.bridgeState,
        messages = uiState.messages,
        query = uiState.query,
        actions = actions,
        modifier = Modifier.weight(1f),
      )
    }
  }
}

@Composable
private fun ChatPanel(
  state: ChatWorkspaceState,
  bridgeState: MobileBridgeUiState,
  messages: List<ChatMessage>,
  query: String,
  actions: ChatWorkspaceActions,
  modifier: Modifier = Modifier,
) {
  val corvusColors = CorvusTheme.colors

  Column(modifier = modifier) {
    BridgeStatusCard(
      bridgeState = bridgeState,
      actions = actions,
      modifier = Modifier.fillMaxWidth(),
    )

    Spacer(modifier = Modifier.height(16.dp))

    Surface(
      modifier = Modifier.fillMaxWidth().weight(1f),
      shape = RoundedCornerShape(20.dp),
      color = corvusColors.glassSurface,
    ) {
      Box(
        modifier =
          Modifier.background(
            brush =
              Brush.verticalGradient(listOf(Color.White.copy(alpha = 0.05f), Color.Transparent))
          )
      ) {
        LazyColumn(
          modifier = Modifier.fillMaxSize().padding(16.dp),
          verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
          items(items = messages, key = { it.id }, contentType = { it.role }) { message ->
            ChatBubble(message = message, modelName = state.modelName)
          }
        }
      }
    }

    Spacer(modifier = Modifier.height(16.dp))

    ChatInputField(
      value = query,
      onValueChange = actions.onQueryChange,
      onSend = actions.onSend,
      placeholder = state.inputPlaceholder,
      enabled = bridgeState.isChatReady,
    )
  }
}
