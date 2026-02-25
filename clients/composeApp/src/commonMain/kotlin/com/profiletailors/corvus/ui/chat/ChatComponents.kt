package com.profiletailors.corvus.ui.chat

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Visibility
import androidx.compose.material.icons.filled.VisibilityOff
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp

internal val ChatPanelShape = RoundedCornerShape(16.dp)
internal val ConfigPanelShape = RoundedCornerShape(16.dp)
internal val EndpointCardShape = RoundedCornerShape(12.dp)
internal val ChatBubbleShape = RoundedCornerShape(14.dp)

internal val HEALTH_DETAILS =
  listOf("Auth: none", "Respuesta: {\"status\": \"ok\", \"paired\": bool, \"runtime\": {...}}")

internal val PAIR_DETAILS =
  listOf(
    "Header requerido: X-Pairing-Code",
    "Sin body JSON",
    "Respuesta OK: {\"paired\": true, \"token\": \"zc_...\"}",
    "Errores: 403 codigo invalido, 429 rate limit/lockout",
  )

internal val WEBHOOK_DETAILS =
  listOf(
    "Body requerido: {\"message\": \"...\"}",
    "Header auth: Authorization: Bearer <token> (si require_pairing=true)",
    "Header opcional adicional: X-Webhook-Secret",
    "Header opcional: X-Idempotency-Key",
    "Respuesta OK: {\"response\": \"...\", \"model\": \"...\"}",
  )

@Immutable data class ChatMessage(val id: Int, val role: ChatRole, val content: String)

enum class ChatRole {
  User,
  Assistant,
}

@Composable
internal fun passwordTextField(
  value: String,
  onValueChange: (String) -> Unit,
  label: String,
  placeholder: String,
  modifier: Modifier = Modifier,
) {
  var isVisible by remember { mutableStateOf(false) }

  OutlinedTextField(
    value = value,
    onValueChange = onValueChange,
    label = { Text(label) },
    placeholder = { Text(placeholder) },
    singleLine = true,
    modifier = modifier.fillMaxWidth(),
    visualTransformation =
      if (isVisible) VisualTransformation.None else PasswordVisualTransformation(),
    trailingIcon = {
      IconButton(onClick = { isVisible = !isVisible }) {
        Icon(
          imageVector = if (isVisible) Icons.Default.VisibilityOff else Icons.Default.Visibility,
          contentDescription = if (isVisible) "Hide" else "Show",
        )
      }
    },
  )
}

@Composable
internal fun endpointsSection(healthUrl: String, pairUrl: String, webhookUrl: String) {
  Column(verticalArrangement = Arrangement.spacedBy(10.dp)) {
    endpointCard(title = "GET /health", subtitle = healthUrl, details = HEALTH_DETAILS)
    endpointCard(title = "POST /pair", subtitle = pairUrl, details = PAIR_DETAILS)
    endpointCard(title = "POST /webhook", subtitle = webhookUrl, details = WEBHOOK_DETAILS)
  }
}

@Composable
internal fun endpointCard(title: String, subtitle: String, details: List<String>) {
  val colors = MaterialTheme.colorScheme
  val outlineStroke =
    remember(colors.outline) { BorderStroke(1.dp, colors.outline.copy(alpha = 0.25f)) }

  Surface(
    modifier = Modifier.fillMaxWidth(),
    shape = EndpointCardShape,
    color = colors.surfaceVariant.copy(alpha = 0.2f),
    border = outlineStroke,
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
internal fun ChatBubble(message: ChatMessage, modelName: String) {
  val isUser = message.role == ChatRole.User
  val colors = MaterialTheme.colorScheme
  val outlineStroke =
    remember(colors.outline) { BorderStroke(1.dp, colors.outline.copy(alpha = 0.2f)) }

  Row(
    modifier = Modifier.fillMaxWidth(),
    horizontalArrangement = if (isUser) Arrangement.End else Arrangement.Start,
  ) {
    Surface(
      modifier = Modifier.widthIn(max = 320.dp),
      shape = ChatBubbleShape,
      color = if (isUser) colors.primaryContainer else colors.surface,
      border = outlineStroke,
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

internal fun endpointUrl(baseUrl: String, path: String): String {
  val normalizedBase = baseUrl.trim().removeSuffix("/")
  if (normalizedBase.isEmpty()) {
    return path
  }
  return "$normalizedBase$path"
}

internal fun buildLocalAssistantReply(
  prompt: String,
  modelName: String,
  gateway: AgentGatewayConfig,
): String {
  val webhook = endpointUrl(gateway.baseUrl, "/webhook")
  val authState = if (gateway.bearerToken.isBlank()) "sin token" else "con token"
  return "[$modelName] Recibido: \"$prompt\". Endpoint objetivo: $webhook ($authState)."
}
