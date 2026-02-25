package com.profiletailors.corvus.ui.chat

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp

@Composable
internal fun ConfigPanel(
  gatewayConfig: AgentGatewayConfig,
  actions: ChatWorkspaceActions,
  modifier: Modifier = Modifier,
) {
  val colors = MaterialTheme.colorScheme
  val borderStroke =
    remember(colors.outline) { BorderStroke(1.dp, colors.outline.copy(alpha = 0.3f)) }

  Surface(
    modifier = modifier.fillMaxWidth(),
    shape = ConfigPanelShape,
    border = borderStroke,
    color = colors.surface,
  ) {
    ConfigSettingsList(gatewayConfig = gatewayConfig, actions = actions)
  }
}

@Composable
internal fun ConfigSettingsList(gatewayConfig: AgentGatewayConfig, actions: ChatWorkspaceActions) {
  val (healthUrl, pairUrl, webhookUrl) =
    remember(gatewayConfig.baseUrl) {
      Triple(
        endpointUrl(gatewayConfig.baseUrl, "/health"),
        endpointUrl(gatewayConfig.baseUrl, "/pair"),
        endpointUrl(gatewayConfig.baseUrl, "/webhook"),
      )
    }

  LazyColumn(
    modifier = Modifier.fillMaxSize().padding(14.dp),
    verticalArrangement = Arrangement.spacedBy(10.dp),
  ) {
    item {
      OutlinedTextField(
        value = gatewayConfig.baseUrl,
        onValueChange = actions.onBaseUrlChange,
        label = { Text("URL base del agente") },
        placeholder = { Text(ChatWorkspaceDefaults.DefaultGatewayBaseUrl) },
        singleLine = true,
        modifier = Modifier.fillMaxWidth(),
      )
    }

    item {
      OutlinedTextField(
        value = gatewayConfig.pairingCode,
        onValueChange = actions.onPairingCodeChange,
        label = { Text("Pairing code (X-Pairing-Code)") },
        placeholder = { Text("Codigo de 6 digitos") },
        singleLine = true,
        modifier = Modifier.fillMaxWidth(),
      )
    }

    item {
      passwordTextField(
        value = gatewayConfig.bearerToken,
        onValueChange = actions.onBearerTokenChange,
        label = "Bearer token",
        placeholder = "zc_...",
      )
    }

    item {
      passwordTextField(
        value = gatewayConfig.webhookSecret,
        onValueChange = actions.onWebhookSecretChange,
        label = "Webhook secret (opcional)",
        placeholder = "X-Webhook-Secret",
      )
    }

    item { endpointsSection(healthUrl = healthUrl, pairUrl = pairUrl, webhookUrl = webhookUrl) }
  }
}
