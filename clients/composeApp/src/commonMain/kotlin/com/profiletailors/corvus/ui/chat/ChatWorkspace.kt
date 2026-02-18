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
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp

private val ChatPanelShape = RoundedCornerShape(16.dp)
private val ConfigPanelShape = RoundedCornerShape(16.dp)
private val EndpointCardShape = RoundedCornerShape(12.dp)
private val ChatBubbleShape = RoundedCornerShape(14.dp)

@Immutable
data class ChatWorkspaceState(
  val modelName: String,
  val inputPlaceholder: String,
  val welcomeMessage: String,
)

@Immutable
data class AgentGatewayConfig(
  val baseUrl: String,
  val pairingCode: String,
  val bearerToken: String,
  val webhookSecret: String,
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

@Immutable private data class ChatMessage(val id: Int, val role: ChatRole, val content: String)

private enum class ChatRole {
  User,
  Assistant,
}

@Composable
fun ChatWorkspace(
  modifier: Modifier = Modifier,
  state: ChatWorkspaceState = ChatWorkspaceDefaults.state(),
) {
  var query by rememberSaveable { mutableStateOf("") }
  var nextId by rememberSaveable { mutableIntStateOf(1) }
  var showConfig by rememberSaveable { mutableStateOf(false) }
  var baseUrl by rememberSaveable { mutableStateOf(ChatWorkspaceDefaults.DefaultGatewayBaseUrl) }
  var pairingCode by rememberSaveable { mutableStateOf("") }
  var bearerToken by rememberSaveable { mutableStateOf("") }
  var webhookSecret by rememberSaveable { mutableStateOf("") }

  val messages =
    remember(state.welcomeMessage) {
      mutableStateListOf(
        ChatMessage(id = 0, role = ChatRole.Assistant, content = state.welcomeMessage)
      )
    }

  fun sendMessage() {
    val prompt = query.trim()
    if (prompt.isBlank()) {
      return
    }

    val gatewayConfig =
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
            gateway = gatewayConfig,
          ),
      )
    )
    nextId += 1
    query = ""
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

  val onToggleConfigLambda = remember { { showConfig = !showConfig } }

  ChatWorkspaceScreen(
    state = state,
    messages = messages,
    query = query,
    showConfig = showConfig,
    gatewayConfig = gatewayConfig,
    onQueryChange = { query = it },
    onSend = ::sendMessage,
    onToggleConfig = onToggleConfigLambda,
    onBaseUrlChange = { baseUrl = it },
    onPairingCodeChange = { pairingCode = it },
    onBearerTokenChange = { bearerToken = it },
    onWebhookSecretChange = { webhookSecret = it },
    modifier = modifier,
  )
}

@Composable
private fun ChatWorkspaceScreen(
  state: ChatWorkspaceState,
  messages: List<ChatMessage>,
  query: String,
  showConfig: Boolean,
  gatewayConfig: AgentGatewayConfig,
  onQueryChange: (String) -> Unit,
  onSend: () -> Unit,
  onToggleConfig: () -> Unit,
  onBaseUrlChange: (String) -> Unit,
  onPairingCodeChange: (String) -> Unit,
  onBearerTokenChange: (String) -> Unit,
  onWebhookSecretChange: (String) -> Unit,
  modifier: Modifier = Modifier,
) {
  val colors = MaterialTheme.colorScheme

  Column(
    modifier =
      modifier.fillMaxSize().background(colors.background).safeContentPadding().padding(16.dp)
  ) {
    ChatHeader(
      modelName = state.modelName,
      showConfig = showConfig,
      onToggleConfig = onToggleConfig,
    )

    Spacer(modifier = Modifier.height(12.dp))

    if (showConfig) {
      Column(modifier = Modifier.fillMaxWidth().weight(1f)) {
        ConfigPanel(
          gatewayConfig = gatewayConfig,
          onBaseUrlChange = onBaseUrlChange,
          onPairingCodeChange = onPairingCodeChange,
          onBearerTokenChange = onBearerTokenChange,
          onWebhookSecretChange = onWebhookSecretChange,
          modifier = Modifier.fillMaxSize(),
        )
      }
    } else {
      Column(modifier = Modifier.fillMaxWidth().weight(1f)) {
        ChatPanel(
          modelName = state.modelName,
          inputPlaceholder = state.inputPlaceholder,
          messages = messages,
          query = query,
          onQueryChange = onQueryChange,
          onSend = onSend,
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
  modelName: String,
  inputPlaceholder: String,
  messages: List<ChatMessage>,
  query: String,
  onQueryChange: (String) -> Unit,
  onSend: () -> Unit,
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
      items(items = messages, key = { it.id }) { message ->
        ChatBubble(message = message, modelName = modelName)
      }
    }
  }

  Spacer(modifier = Modifier.height(12.dp))

  Row(modifier = Modifier.fillMaxWidth(), verticalAlignment = Alignment.Bottom) {
    TextField(
      value = query,
      onValueChange = onQueryChange,
      modifier = Modifier.weight(1f),
      placeholder = { Text(inputPlaceholder) },
      keyboardOptions = KeyboardOptions(imeAction = ImeAction.Send),
      keyboardActions = KeyboardActions(onSend = { onSend() }),
      maxLines = 4,
    )

    Spacer(modifier = Modifier.width(8.dp))

    val isSendEnabled = query.isNotBlank()
    Button(onClick = onSend, enabled = isSendEnabled, modifier = Modifier.height(56.dp)) {
      Text("Send")
    }
  }
}

@Composable
private fun ConfigPanel(
  gatewayConfig: AgentGatewayConfig,
  onBaseUrlChange: (String) -> Unit,
  onPairingCodeChange: (String) -> Unit,
  onBearerTokenChange: (String) -> Unit,
  onWebhookSecretChange: (String) -> Unit,
  modifier: Modifier = Modifier,
) {
  val (healthUrl, pairUrl, webhookUrl) =
    remember(gatewayConfig.baseUrl) {
      Triple(
        endpointUrl(gatewayConfig.baseUrl, "/health"),
        endpointUrl(gatewayConfig.baseUrl, "/pair"),
        endpointUrl(gatewayConfig.baseUrl, "/webhook"),
      )
    }

  Surface(
    modifier = modifier.fillMaxWidth(),
    shape = ConfigPanelShape,
    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.3f)),
    color = MaterialTheme.colorScheme.surface,
  ) {
    LazyColumn(
      modifier = Modifier.fillMaxSize().padding(14.dp),
      verticalArrangement = Arrangement.spacedBy(10.dp),
    ) {
      item {
        OutlinedTextField(
          value = gatewayConfig.baseUrl,
          onValueChange = onBaseUrlChange,
          label = { Text("URL base del agente") },
          placeholder = { Text(ChatWorkspaceDefaults.DefaultGatewayBaseUrl) },
          singleLine = true,
          modifier = Modifier.fillMaxWidth(),
        )
      }

      item {
        OutlinedTextField(
          value = gatewayConfig.pairingCode,
          onValueChange = onPairingCodeChange,
          label = { Text("Pairing code (X-Pairing-Code)") },
          placeholder = { Text("Codigo de 6 digitos") },
          singleLine = true,
          modifier = Modifier.fillMaxWidth(),
        )
      }

      item {
        OutlinedTextField(
          value = gatewayConfig.bearerToken,
          onValueChange = onBearerTokenChange,
          label = { Text("Bearer token") },
          placeholder = { Text("zc_...") },
          singleLine = true,
          modifier = Modifier.fillMaxWidth(),
        )
      }

      item {
        OutlinedTextField(
          value = gatewayConfig.webhookSecret,
          onValueChange = onWebhookSecretChange,
          label = { Text("Webhook secret (opcional)") },
          placeholder = { Text("X-Webhook-Secret") },
          singleLine = true,
          modifier = Modifier.fillMaxWidth(),
        )
      }

      item {
        EndpointCard(
          title = "GET /health",
          subtitle = healthUrl,
          details =
            listOf(
              "Auth: none",
              "Respuesta: {\"status\": \"ok\", \"paired\": bool, \"runtime\": {...}}",
            ),
        )
      }

      item {
        EndpointCard(
          title = "POST /pair",
          subtitle = pairUrl,
          details =
            listOf(
              "Header requerido: X-Pairing-Code",
              "Sin body JSON",
              "Respuesta OK: {\"paired\": true, \"token\": \"zc_...\"}",
              "Errores: 403 codigo invalido, 429 rate limit/lockout",
            ),
        )
      }

      item {
        EndpointCard(
          title = "POST /webhook",
          subtitle = webhookUrl,
          details =
            listOf(
              "Body requerido: {\"message\": \"...\"}",
              "Header auth: Authorization: Bearer <token> (si require_pairing=true)",
              "Header opcional adicional: X-Webhook-Secret",
              "Header opcional: X-Idempotency-Key",
              "Respuesta OK: {\"response\": \"...\", \"model\": \"...\"}",
            ),
        )
      }
    }
  }
}

@Composable
private fun EndpointCard(title: String, subtitle: String, details: List<String>) {
  Surface(
    modifier = Modifier.fillMaxWidth(),
    shape = EndpointCardShape,
    color = MaterialTheme.colorScheme.surfaceVariant.copy(alpha = 0.2f),
    border = BorderStroke(1.dp, MaterialTheme.colorScheme.outline.copy(alpha = 0.25f)),
  ) {
    Column(
      modifier = Modifier.fillMaxWidth().padding(12.dp),
      verticalArrangement = Arrangement.spacedBy(6.dp),
    ) {
      Text(
        text = title,
        style = MaterialTheme.typography.titleSmall,
        color = MaterialTheme.colorScheme.onSurface,
        fontWeight = FontWeight.SemiBold,
      )
      Text(
        text = subtitle,
        style = MaterialTheme.typography.labelMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
      )
      details.forEach { line ->
        Text(
          text = "- $line",
          style = MaterialTheme.typography.bodySmall,
          color = MaterialTheme.colorScheme.onSurface,
        )
      }
    }
  }
}

@Composable
private fun ChatBubble(message: ChatMessage, modelName: String) {
  val isUser = message.role == ChatRole.User
  val colors = MaterialTheme.colorScheme

  Row(
    modifier = Modifier.fillMaxWidth(),
    horizontalArrangement = if (isUser) Arrangement.End else Arrangement.Start,
  ) {
    Surface(
      modifier = Modifier.widthIn(max = 320.dp),
      shape = ChatBubbleShape,
      color = if (isUser) colors.primaryContainer else colors.surface,
      border = BorderStroke(1.dp, colors.outline.copy(alpha = 0.2f)),
    ) {
      Column(modifier = Modifier.fillMaxWidth().padding(horizontal = 12.dp, vertical = 10.dp)) {
        Text(
          text = if (isUser) "You" else modelName,
          style = MaterialTheme.typography.labelSmall,
          color = colors.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.height(4.dp))
        Text(
          text = message.content,
          style = MaterialTheme.typography.bodyMedium,
          color = if (isUser) colors.onPrimaryContainer else colors.onSurface,
        )
      }
    }
  }
}

private fun endpointUrl(baseUrl: String, path: String): String {
  val normalizedBase = baseUrl.trim().removeSuffix("/")
  if (normalizedBase.isEmpty()) {
    return path
  }
  return "$normalizedBase$path"
}

private fun buildLocalAssistantReply(
  prompt: String,
  modelName: String,
  gateway: AgentGatewayConfig,
): String {
  val webhook = endpointUrl(gateway.baseUrl, "/webhook")
  val authState = if (gateway.bearerToken.isBlank()) "sin token" else "con token"
  return "[$modelName] Recibido: \"$prompt\". Endpoint objetivo: $webhook ($authState)."
}
