package com.profiletailors.corvus.ui.chat

import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Visibility
import androidx.compose.material.icons.filled.VisibilityOff
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import com.profiletailors.corvus.ui.theme.CorvusColors

// ============================================================================
// Corvus Config Panel - Futuristic Style
// ============================================================================

private val HEALTH_DETAILS =
  listOf("Auth: none", "Response: {\"status\": \"ok\", \"paired\": bool}")

private val PAIR_DETAILS =
  listOf(
    "Header: X-Pairing-Code",
    "Response: {\"paired\": true, \"token\": \"zc_...\"}",
    "Errors: 403 invalid, 429 rate limit",
  )

private val WEBHOOK_DETAILS =
  listOf(
    "Body: {\"message\": \"...\"}",
    "Auth: Bearer <token>",
    "Response: {\"response\": \"...\"}",
  )

// ============================================================================
// Config Panel Main
// ============================================================================

@Composable
internal fun ConfigPanel(
  gatewayConfig: AgentGatewayConfig,
  actions: ChatWorkspaceActions,
  modifier: Modifier = Modifier,
) {
  val colors = MaterialTheme.colorScheme

  Surface(
    modifier = modifier.fillMaxWidth(),
    shape = RoundedCornerShape(20.dp),
    color = CorvusColors.glassSurface,
  ) {
    Box(
      modifier =
        Modifier.background(
          brush =
            Brush.verticalGradient(
              colors = listOf(Color.White.copy(alpha = 0.05f), Color.Transparent)
            )
        )
    ) {
      ConfigSettingsList(gatewayConfig = gatewayConfig, actions = actions)
    }
  }
}

// ============================================================================
// Config Settings List
// ============================================================================

@Composable
internal fun ConfigSettingsList(gatewayConfig: AgentGatewayConfig, actions: ChatWorkspaceActions) {
  val healthUrl = remember(gatewayConfig.baseUrl) { endpointUrl(gatewayConfig.baseUrl, "/health") }
  val pairUrl = remember(gatewayConfig.baseUrl) { endpointUrl(gatewayConfig.baseUrl, "/pair") }
  val webhookUrl =
    remember(gatewayConfig.baseUrl) { endpointUrl(gatewayConfig.baseUrl, "/webhook") }

  LazyColumn(
    modifier = Modifier.fillMaxSize().padding(20.dp),
    verticalArrangement = Arrangement.spacedBy(20.dp),
  ) {
    // Header
    item {
      Column {
        Text(
          text = "Gateway Connection",
          style = MaterialTheme.typography.titleLarge,
          fontWeight = FontWeight.Bold,
          color = MaterialTheme.colorScheme.onSurface,
        )
        Spacer(modifier = Modifier.height(4.dp))
        Text(
          text = "Configure your Corvus runtime endpoint",
          style = MaterialTheme.typography.bodyMedium,
          color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
      }
    }

    // Connection Status
    item {
      StatusIndicator(
        connected = gatewayConfig.baseUrl.isNotBlank(),
        modifier = Modifier.fillMaxWidth(),
      )
    }

    // Gateway URL Field
    item {
      Column {
        Text(
          text = "Gateway URL",
          style = MaterialTheme.typography.labelMedium,
          fontWeight = FontWeight.Medium,
          color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.height(8.dp))
        OutlinedTextField(
          value = gatewayConfig.baseUrl,
          onValueChange = actions.onBaseUrlChange,
          placeholder = { Text(ChatWorkspaceDefaults.DefaultGatewayBaseUrl) },
          singleLine = true,
          modifier = Modifier.fillMaxWidth(),
          colors =
            OutlinedTextFieldDefaults.colors(
              focusedBorderColor = CorvusColors.glowPurple,
              unfocusedBorderColor = MaterialTheme.colorScheme.outline.copy(alpha = 0.3f),
              focusedTextColor = MaterialTheme.colorScheme.onSurface,
              unfocusedTextColor = MaterialTheme.colorScheme.onSurface,
            ),
          shape = RoundedCornerShape(12.dp),
        )
      }
    }

    // Pairing Code Field
    item {
      Column {
        Text(
          text = "Pairing Code",
          style = MaterialTheme.typography.labelMedium,
          fontWeight = FontWeight.Medium,
          color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
        Spacer(modifier = Modifier.height(8.dp))
        OutlinedTextField(
          value = gatewayConfig.pairingCode,
          onValueChange = actions.onPairingCodeChange,
          placeholder = { Text("6-digit code") },
          singleLine = true,
          modifier = Modifier.fillMaxWidth(),
          colors =
            OutlinedTextFieldDefaults.colors(
              focusedBorderColor = CorvusColors.glowCyan,
              unfocusedBorderColor = MaterialTheme.colorScheme.outline.copy(alpha = 0.3f),
              focusedTextColor = MaterialTheme.colorScheme.onSurface,
              unfocusedTextColor = MaterialTheme.colorScheme.onSurface,
            ),
          shape = RoundedCornerShape(12.dp),
        )
      }
    }

    // Bearer Token Field
    item {
      PasswordTextField(
        value = gatewayConfig.bearerToken,
        onValueChange = actions.onBearerTokenChange,
        label = "Bearer Token",
        placeholder = "zc_...",
      )
    }

    // Webhook Secret Field
    item {
      PasswordTextField(
        value = gatewayConfig.webhookSecret,
        onValueChange = actions.onWebhookSecretChange,
        label = "Webhook Secret (Optional)",
        placeholder = "X-Webhook-Secret",
      )
    }

    // Action Buttons
    item {
      Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        // Test Connection Button (Outline)
        OutlinedButton(
          onClick = { /* Test connection */ },
          modifier = Modifier.weight(1f),
          shape = RoundedCornerShape(12.dp),
          colors = ButtonDefaults.outlinedButtonColors(contentColor = CorvusColors.glowCyan),
        ) {
          Text(text = "Test Connection", fontWeight = FontWeight.Medium)
        }

        // Save Button (Gradient)
        GradientButton(
          text = "Save",
          onClick = { /* Save config */ },
          modifier = Modifier.weight(1f),
        )
      }
    }

    // Divider
    item {
      Box(
        modifier =
          Modifier.fillMaxWidth()
            .height(1.dp)
            .background(
              brush =
                Brush.horizontalGradient(
                  colors = listOf(Color.Transparent, CorvusColors.glassOverlay, Color.Transparent)
                )
            )
      )
    }

    // API Documentation Section
    item {
      Column {
        Text(
          text = "API Reference",
          style = MaterialTheme.typography.titleMedium,
          fontWeight = FontWeight.SemiBold,
          color = MaterialTheme.colorScheme.onSurface,
        )
        Spacer(modifier = Modifier.height(16.dp))
        endpointCard(title = "GET /health", subtitle = healthUrl, details = HEALTH_DETAILS)
        Spacer(modifier = Modifier.height(12.dp))
        endpointCard(title = "POST /pair", subtitle = pairUrl, details = PAIR_DETAILS)
        Spacer(modifier = Modifier.height(12.dp))
        endpointCard(title = "POST /webhook", subtitle = webhookUrl, details = WEBHOOK_DETAILS)
      }
    }

    // Bottom padding
    item { Spacer(modifier = Modifier.height(32.dp)) }
  }
}

// ============================================================================
// Password Text Field - Futuristic Style
// ============================================================================

@Composable
private fun PasswordTextField(
  value: String,
  onValueChange: (String) -> Unit,
  label: String,
  placeholder: String,
) {
  var isVisible by remember { mutableStateOf(false) }
  val colors = MaterialTheme.colorScheme

  Column {
    Text(
      text = label,
      style = MaterialTheme.typography.labelMedium,
      fontWeight = FontWeight.Medium,
      color = colors.onSurfaceVariant,
    )
    Spacer(modifier = Modifier.height(8.dp))
    OutlinedTextField(
      value = value,
      onValueChange = onValueChange,
      label = { Text(label) },
      placeholder = { Text(placeholder) },
      singleLine = true,
      modifier = Modifier.fillMaxWidth(),
      visualTransformation =
        if (isVisible) VisualTransformation.None else PasswordVisualTransformation(),
      colors =
        OutlinedTextFieldDefaults.colors(
          focusedBorderColor = CorvusColors.glowPurple,
          unfocusedBorderColor = colors.outline.copy(alpha = 0.3f),
          focusedTextColor = colors.onSurface,
          unfocusedTextColor = colors.onSurface,
        ),
      trailingIcon = {
        IconButton(onClick = { isVisible = !isVisible }) {
          Icon(
            imageVector = if (isVisible) Icons.Default.VisibilityOff else Icons.Default.Visibility,
            contentDescription = if (isVisible) "Hide" else "Show",
            tint = colors.onSurfaceVariant,
          )
        }
      },
      shape = RoundedCornerShape(12.dp),
    )
  }
}
