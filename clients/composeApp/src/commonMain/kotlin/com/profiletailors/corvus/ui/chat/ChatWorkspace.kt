@file:Suppress("FunctionNaming")

package com.profiletailors.corvus.ui.chat

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeContentPadding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TextField
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
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp

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
)

object ChatWorkspaceDefaults {
  const val DefaultAgentName = "Corvus Agent"
  const val DefaultGatewayBaseUrl = "http://127.0.0.1:3000"

  fun state(modelName: String = DefaultAgentName): ChatWorkspaceState =
    ChatWorkspaceState(
      modelName = modelName,
      inputPlaceholder = "Escribe un mensaje...",
      welcomeMessage = "Hola, soy $modelName. En que puedo ayudarte?",
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
  var baseUrl by rememberSaveable { mutableStateOf(ChatWorkspaceDefaults.DefaultGatewayBaseUrl) }
  var pairingCode by remember { mutableStateOf("") }
  var bearerToken by remember { mutableStateOf("") }
  var webhookSecret by remember { mutableStateOf("") }

  val messages =
    remember(state.welcomeMessage) {
      mutableStateListOf(
        ChatMessage(id = 0, role = ChatRole.Assistant, content = state.welcomeMessage)
      )
    }

  val gatewayConfig =
    remember(baseUrl, pairingCode, bearerToken, webhookSecret) {
      AgentGatewayConfig(
        baseUrl = baseUrl,
        pairingCode = pairingCode,
        bearerToken = bearerToken,
        webhookSecret = webhookSecret,
      )
    }

  fun sendMessage() {
    val prompt = query.trim()
    if (prompt.isBlank()) {
      return
    }

    // Performance: Always read the latest configuration at invocation time
    // to avoid stale capture when called from remembered actions.
    val currentGatewayConfig =
      AgentGatewayConfig(
        baseUrl = baseUrl,
        pairingCode = pairingCode,
        bearerToken = bearerToken,
        webhookSecret = webhookSecret,
      )

    messages.add(ChatMessage(id = nextId, role = ChatRole.User, content = prompt))
    nextId += 1

    messages.add(
      ChatMessage(
        id = nextId,
        role = ChatRole.Assistant,
        content =
          buildLocalAssistantReply(
            prompt = prompt,
            modelName = state.modelName,
            gateway = currentGatewayConfig,
          ),
      )
    )
    nextId += 1
    query = ""
  }

  val actions =
    remember(state) {
      ChatWorkspaceActions(
        onQueryChange = { query = it },
        onSend = ::sendMessage,
        onToggleConfig = { showConfig = !showConfig },
        onBaseUrlChange = { baseUrl = it },
        onPairingCodeChange = { pairingCode = it },
        onBearerTokenChange = { bearerToken = it },
        onWebhookSecretChange = { webhookSecret = it },
      )
    }

  val uiState =
    remember(state, query, showConfig) {
      ChatUiState(
        workspaceState = state,
        messages = messages,
        query = query,
        showConfig = showConfig,
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

  // Performance: Remember the modifier chain to avoid redundant allocations
  // and chain reconstructions on every recomposition (e.g. during typing).
  val screenModifier =
    remember(modifier, colors.background) {
      modifier.fillMaxSize().background(colors.background).safeContentPadding().padding(16.dp)
    }

  Column(modifier = screenModifier) {
    ChatHeader(
      modelName = uiState.workspaceState.modelName,
      showConfig = uiState.showConfig,
      onToggleConfig = actions.onToggleConfig,
    )

    Spacer(modifier = Modifier.height(12.dp))

    if (uiState.showConfig) {
      Column(modifier = Modifier.fillMaxWidth().weight(1f)) {
        ConfigPanel(
          gatewayConfig = gatewayConfig,
          actions = actions,
          modifier = Modifier.fillMaxSize(),
        )
      }
    } else {
      Column(modifier = Modifier.fillMaxWidth().weight(1f)) {
        ChatPanel(
          state = uiState.workspaceState,
          messages = uiState.messages,
          query = uiState.query,
          actions = actions,
          modifier = Modifier.fillMaxSize(),
        )
      }
    }
  }
}

@Composable
private fun ChatHeader(modelName: String, showConfig: Boolean, onToggleConfig: () -> Unit) {
  Row(
    modifier = Modifier.fillMaxWidth(),
    horizontalArrangement = Arrangement.SpaceBetween,
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Column {
      Text(
        text = modelName,
        style = MaterialTheme.typography.headlineSmall,
        color = MaterialTheme.colorScheme.onBackground,
      )
      Text(
        text = if (showConfig) "Configuracion del gateway" else "Simple AI chat",
        style = MaterialTheme.typography.labelMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
      )
    }

    TextButton(onClick = onToggleConfig) { Text(if (showConfig) "Volver al chat" else "Config") }
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
  val colors = MaterialTheme.colorScheme
  val borderStroke =
    remember(colors.outline) { BorderStroke(1.dp, colors.outline.copy(alpha = 0.3f)) }

  Surface(
    modifier = modifier.fillMaxWidth(),
    shape = ChatPanelShape,
    color = colors.surfaceVariant.copy(alpha = 0.32f),
    border = borderStroke,
  ) {
    LazyColumn(
      modifier = Modifier.fillMaxSize().padding(12.dp),
      verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
      // Performance: contentType helps LazyColumn reuse item slots efficiently.
      items(items = messages, key = { it.id }, contentType = { it.role }) { message ->
        ChatBubble(message = message, modelName = state.modelName)
      }
    }
  }

  Spacer(modifier = Modifier.height(12.dp))

  Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.Bottom) {
    TextField(
      value = query,
      onValueChange = actions.onQueryChange,
      modifier = Modifier.weight(1f),
      placeholder = { Text(state.inputPlaceholder) },
      keyboardOptions = KeyboardOptions(imeAction = ImeAction.Send),
      keyboardActions = KeyboardActions(onSend = { actions.onSend() }),
      maxLines = 4,
    )

    Spacer(modifier = Modifier.width(8.dp))

    // Performance: Use remember(query) to avoid redundant blank checks on every
    // recomposition when the query hasn't changed.
    val isSendEnabled = remember(query) { query.trim().isNotBlank() }
    Button(onClick = actions.onSend, enabled = isSendEnabled, modifier = Modifier.height(56.dp)) {
      Text("Send")
    }
  }
}
