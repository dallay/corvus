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

@Stable
data class ChatUiState(
  val workspaceState: ChatWorkspaceState,
  val messages: List<ChatMessage>,
  val query: String,
  val showConfig: Boolean,
  val isGatewayConfigured: Boolean,
)

@Immutable
data class AgentGatewayConfig(
  val baseUrl: String,
  val pairingCode: String,
  val bearerToken: String,
  val webhookSecret: String,
)

@Stable
data class ChatWorkspaceActions(
  val onQueryChange: (String) -> Unit,
  val onSend: () -> Unit,
  val onToggleConfig: () -> Unit,
  val onBaseUrlChange: (String) -> Unit,
  val onPairingCodeChange: (String) -> Unit,
  val onBearerTokenChange: (String) -> Unit,
  val onWebhookSecretChange: (String) -> Unit,
  val onTestConnection: (AgentGatewayConfig) -> Unit,
  val onSaveGatewayConfig: (AgentGatewayConfig) -> Unit,
)

object ChatWorkspaceDefaults {
  const val DefaultAgentName = "Corvus"
  const val DefaultGatewayBaseUrl = "http://127.0.0.1:3000"

  fun state(modelName: String = DefaultAgentName): ChatWorkspaceState =
    ChatWorkspaceState(
      modelName = modelName,
      inputPlaceholder = "Ask your AI agent anything...",
      welcomeMessage = "Hello! I'm Corvus, your always-on AI agent. How can I help you today?",
    )
}

@Composable
fun ChatWorkspace(
  modifier: Modifier = Modifier,
  state: ChatWorkspaceState = ChatWorkspaceDefaults.state(),
) {
  var query by remember { mutableStateOf("") }
  var nextId by rememberSaveable { mutableIntStateOf(1) }
  var showConfig by rememberSaveable { mutableStateOf(false) }

  var draftBaseUrl by rememberSaveable {
    mutableStateOf(ChatWorkspaceDefaults.DefaultGatewayBaseUrl)
  }
  var draftPairingCode by rememberSaveable { mutableStateOf("") }
  var draftBearerToken by rememberSaveable { mutableStateOf("") }
  var draftWebhookSecret by rememberSaveable { mutableStateOf("") }

  var savedBaseUrl by rememberSaveable {
    mutableStateOf(ChatWorkspaceDefaults.DefaultGatewayBaseUrl)
  }
  var savedPairingCode by rememberSaveable { mutableStateOf("") }
  var savedBearerToken by rememberSaveable { mutableStateOf("") }
  var savedWebhookSecret by rememberSaveable { mutableStateOf("") }
  var isGatewayConfigured by rememberSaveable {
    mutableStateOf(isGatewayConfigConfigured(ChatWorkspaceDefaults.DefaultGatewayBaseUrl))
  }

  val messages =
    remember(state.welcomeMessage) {
      mutableStateListOf(
        ChatMessage(id = 0, role = ChatRole.Assistant, content = state.welcomeMessage)
      )
    }

  val gatewayConfig =
    remember(draftBaseUrl, draftPairingCode, draftBearerToken, draftWebhookSecret) {
      AgentGatewayConfig(
        baseUrl = draftBaseUrl,
        pairingCode = draftPairingCode,
        bearerToken = draftBearerToken,
        webhookSecret = draftWebhookSecret,
      )
    }

  val savedGatewayConfig =
    remember(savedBaseUrl, savedPairingCode, savedBearerToken, savedWebhookSecret) {
      AgentGatewayConfig(
        baseUrl = savedBaseUrl,
        pairingCode = savedPairingCode,
        bearerToken = savedBearerToken,
        webhookSecret = savedWebhookSecret,
      )
    }

  fun sendMessage() {
    val prompt = query.trim()
    if (prompt.isBlank()) return

    messages.add(ChatMessage(id = nextId, role = ChatRole.User, content = prompt))
    nextId += 1

    messages.add(
      ChatMessage(
        id = nextId,
        role = ChatRole.Assistant,
        content = buildLocalAssistantReply(prompt, state.modelName, savedGatewayConfig),
      )
    )
    nextId += 1
    query = ""
  }

  val actions = remember {
    ChatWorkspaceActions(
      onQueryChange = { query = it },
      onSend = ::sendMessage,
      onToggleConfig = { showConfig = !showConfig },
      onBaseUrlChange = { draftBaseUrl = it },
      onPairingCodeChange = { draftPairingCode = it },
      onBearerTokenChange = { draftBearerToken = it },
      onWebhookSecretChange = { draftWebhookSecret = it },
      onTestConnection = { config ->
        isGatewayConfigured = isGatewayConfigConfigured(config.baseUrl)
      },
      onSaveGatewayConfig = { config ->
        savedBaseUrl = config.baseUrl
        savedPairingCode = config.pairingCode
        savedBearerToken = config.bearerToken
        savedWebhookSecret = config.webhookSecret
        isGatewayConfigured = isGatewayConfigConfigured(config.baseUrl)
      },
    )
  }

  val uiState =
    remember(state, query, showConfig, isGatewayConfigured) {
      ChatUiState(
        workspaceState = state,
        messages = messages,
        query = query,
        showConfig = showConfig,
        isGatewayConfigured = isGatewayConfigured,
      )
    }

  ChatWorkspaceScreen(
    uiState = uiState,
    gatewayConfig = gatewayConfig,
    actions = actions,
    modifier = modifier,
  )
}

@Composable
private fun ChatWorkspaceScreen(
  uiState: ChatUiState,
  gatewayConfig: AgentGatewayConfig,
  actions: ChatWorkspaceActions,
  modifier: Modifier = Modifier,
) {
  val colors = MaterialTheme.colorScheme
  val corvusColors = CorvusTheme.colors

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
      showConfig = uiState.showConfig,
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

    if (uiState.showConfig) {
      ConfigPanel(
        gatewayConfig = gatewayConfig,
        isGatewayConfigured = uiState.isGatewayConfigured,
        actions = actions,
        modifier = Modifier.weight(1f),
      )
    } else {
      ChatPanel(
        state = uiState.workspaceState,
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
  messages: List<ChatMessage>,
  query: String,
  actions: ChatWorkspaceActions,
  modifier: Modifier = Modifier,
) {
  val corvusColors = CorvusTheme.colors

  Column(modifier = modifier) {
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
    )
  }
}

private fun isGatewayConfigConfigured(baseUrl: String): Boolean {
  val trimmed = baseUrl.trim()
  return trimmed.startsWith("http://") || trimmed.startsWith("https://")
}
