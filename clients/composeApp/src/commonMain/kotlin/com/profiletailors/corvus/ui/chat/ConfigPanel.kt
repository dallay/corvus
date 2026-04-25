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
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.profiletailors.corvus.runtime.RuntimeSession
import com.profiletailors.corvus.ui.theme.CorvusTheme

@Suppress("LongParameterList") // Composable parameters are not reducible without losing clarity
@Composable
internal fun ConfigPanel(
  bridgeState: MobileBridgeUiState,
  resumableSessions: List<RuntimeSession>,
  activeSessionId: String?,
  targetLabel: String?,
  actions: ChatWorkspaceActions,
  modifier: Modifier = Modifier,
) {
  val corvusColors = CorvusTheme.colors

  Surface(
    modifier = modifier.fillMaxWidth(),
    shape = RoundedCornerShape(20.dp),
    color = corvusColors.glassSurface,
  ) {
    Box(
      modifier =
        Modifier.background(
          brush = Brush.verticalGradient(listOf(Color.White.copy(alpha = 0.05f), Color.Transparent))
        )
    ) {
      configSettingsList(
        bridgeState = bridgeState,
        resumableSessions = resumableSessions,
        activeSessionId = activeSessionId,
        targetLabel = targetLabel,
        actions = actions,
      )
    }
  }
}

@Composable
internal fun configSettingsList(
  bridgeState: MobileBridgeUiState,
  resumableSessions: List<RuntimeSession>,
  activeSessionId: String?,
  targetLabel: String?,
  actions: ChatWorkspaceActions,
) {
  val corvusColors = CorvusTheme.colors
  val onboardingState = bridgeState.onboardingState
  val detailLines = buildDiagnosticsLines(bridgeState)

  LazyColumn(
    modifier = Modifier.fillMaxSize().padding(20.dp),
    verticalArrangement = Arrangement.spacedBy(20.dp),
  ) {
    item {
      Column {
        // Client-first: Use "Connection" instead of "Bridge Linking"
        Text(
          text = "Runtime Connection",
          style = MaterialTheme.typography.titleLarge,
          fontWeight = FontWeight.Bold,
          color = MaterialTheme.colorScheme.onSurface,
        )
        Spacer(modifier = Modifier.height(4.dp))
        Text(
          text = bridgeStateHeadline(bridgeState),
          style = MaterialTheme.typography.bodyMedium,
          color = MaterialTheme.colorScheme.onSurfaceVariant,
        )
      }
    }

    item {
      StatusIndicator(
        active = bridgeState.snapshot.linkEstablished && bridgeState.snapshot.sessionCapable,
        label = onboardingStateLabel(onboardingState),
        modifier = Modifier.fillMaxWidth(),
      )
    }

    item {
      diagnosticsCard(
        title = "Runtime target",
        subtitle = targetLabel ?: bridgeState.platformName,
        details = detailLines,
      )
    }

    item {
      diagnosticsCard(
        title = "Safe diagnostics",
        subtitle = "Parity-critical bridge details only",
        details = buildSafeDiagnosticLines(bridgeState, targetLabel),
      )
    }

    item {
      SessionSelectionCard(
        sessions = resumableSessions,
        activeSessionId = activeSessionId,
        onResumeSession = actions.bridge.onResumeSession,
        modifier = Modifier.fillMaxWidth(),
      )
    }

    item {
      Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        if (onboardingState.canRetry) {
          OutlinedButton(
            onClick = actions.bridge.onRetryBridge,
            modifier = Modifier.weight(1f),
            shape = RoundedCornerShape(12.dp),
            colors = ButtonDefaults.outlinedButtonColors(contentColor = corvusColors.glowCyan),
          ) {
            Text(text = "Retry", fontWeight = FontWeight.Medium)
          }
        }

        when (onboardingState.status) {
          MobileOnboardingStatus.TRUST_PENDING -> {
            GradientButton(
              text = "Link app",
              onClick = actions.bridge.onLinkSurface,
              modifier = Modifier.weight(1f),
            )
          }

          MobileOnboardingStatus.SESSION_PENDING -> {
            GradientButton(
              text = "Start session",
              onClick = actions.bridge.onStartSession,
              modifier = Modifier.weight(1f),
            )
          }

          MobileOnboardingStatus.SESSION_READY -> {
            GradientButton(
              text = "Disconnect",
              onClick = actions.bridge.onDisconnectReset,
              modifier = Modifier.weight(1f),
            )
          }

          MobileOnboardingStatus.BLOCKED -> {
            if (bridgeState.snapshot.environmentSupported) {
              GradientButton(
                text = "Relink",
                onClick = actions.bridge.onDisconnectReset,
                modifier = Modifier.weight(1f),
              )
            }
          }

          MobileOnboardingStatus.TARGET_SELECTED,
          MobileOnboardingStatus.RECOVERY,
          MobileOnboardingStatus.RUNTIME_PATH_CONFIRMED,
          MobileOnboardingStatus.TRANSPORT_CONNECTING -> Unit
        }
      }
    }

    item {
      Box(
        modifier =
          Modifier.fillMaxWidth()
            .height(1.dp)
            .background(
              brush =
                Brush.horizontalGradient(
                  listOf(Color.Transparent, corvusColors.glassOverlay, Color.Transparent)
                )
            )
      )
    }

    item {
      diagnosticsCard(
        title = "Reset options",
        subtitle = "Relink or disconnect without exposing unsafe controls",
        details = buildResetOptionLines(),
      )
    }

    item { Spacer(modifier = Modifier.height(32.dp)) }
  }
}

private fun buildDiagnosticsLines(bridgeState: MobileBridgeUiState): List<String> = buildList {
  add("Runtime available: ${yesNo(bridgeState.snapshot.runtimeAvailable)}")
  add("Link established: ${yesNo(bridgeState.snapshot.linkEstablished)}")
  add("Session capable: ${yesNo(bridgeState.snapshot.sessionCapable)}")
  add("Session id: ${bridgeState.snapshot.sessionId ?: "No active or resumable session"}")
}

internal fun buildSafeDiagnosticLines(
  bridgeState: MobileBridgeUiState,
  targetLabel: String?,
): List<String> = buildList {
  // Client-first: Show target info, not local bridge
  add("Target: ${targetLabel ?: "Not configured"}")
  add("Transport: Runtime endpoint")
  add("Recommended timeout: 30 seconds")
  add(bridgeStateRecovery(bridgeState))
}

internal fun buildResetOptionLines(): List<String> =
  listOf(
    // Client-first: Retry connection checks, not bridge checks
    "Retry connection checks after runtime availability changes",
    // Client-first: Don't mention CLI bridge
    "Reconfigure through the supported connection methods",
    "Disconnect & reset clears the active session and connection target metadata",
  )

private fun yesNo(value: Boolean): String = if (value) "Yes" else "No"
