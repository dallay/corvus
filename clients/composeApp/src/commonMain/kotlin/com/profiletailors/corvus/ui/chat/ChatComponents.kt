package com.profiletailors.corvus.ui.chat

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.profiletailors.corvus.runtime.RuntimeApprovalRequest
import com.profiletailors.corvus.runtime.RuntimeSession
import com.profiletailors.corvus.ui.theme.CorvusColorPalette
import com.profiletailors.corvus.ui.theme.CorvusTheme

internal val ChatPanelShape = RoundedCornerShape(20.dp)
internal val ConfigPanelShape = RoundedCornerShape(20.dp)
internal val DiagnosticsCardShape = RoundedCornerShape(16.dp)
internal val ChatBubbleShape = RoundedCornerShape(18.dp)

private const val SETTINGS_CONTENT_DESCRIPTION = "Settings"
private const val RECOVERY_REQUIRED_LABEL = "Recovery required"
private const val TARGET_SELECTED_LABEL = "Target selected"

@Immutable data class ChatMessage(val id: Int, val role: ChatRole, val content: String)

enum class ChatRole {
  User,
  Assistant,
}

@Immutable
private data class ChatBubblePalette(val background: Color, val accent: Color, val shadow: Color)

@Composable
fun GlassSurface(modifier: Modifier = Modifier, content: @Composable () -> Unit) {
  val corvusColors = CorvusTheme.colors

  Surface(
    modifier = modifier,
    shape = RoundedCornerShape(20.dp),
    color = corvusColors.glassSurface,
    tonalElevation = 0.dp,
  ) {
    Box(
      modifier =
        Modifier.background(
          brush = Brush.verticalGradient(listOf(Color.White.copy(alpha = 0.1f), Color.Transparent))
        )
    ) {
      content()
    }
  }
}

@Composable
fun GradientButton(
  text: String,
  onClick: () -> Unit,
  modifier: Modifier = Modifier,
  enabled: Boolean = true,
) {
  val corvusColors = CorvusTheme.colors
  val gradient =
    remember(corvusColors.gradientPrimary) {
      Brush.horizontalGradient(corvusColors.gradientPrimary)
    }
  val alpha by
    animateFloatAsState(
      targetValue = if (enabled) 1f else 0.5f,
      animationSpec = tween(300),
      label = "buttonAlpha",
    )

  Box(
    modifier =
      modifier
        .shadow(
          elevation = 8.dp,
          shape = RoundedCornerShape(14.dp),
          spotColor = corvusColors.glowPurple.copy(alpha = 0.3f),
        )
        .clip(RoundedCornerShape(14.dp))
        .clickable(enabled = enabled, onClick = onClick)
        .background(gradient)
        .padding(horizontal = 24.dp, vertical = 12.dp),
    contentAlignment = Alignment.Center,
  ) {
    Text(
      text = text,
      style = MaterialTheme.typography.labelLarge,
      fontWeight = FontWeight.SemiBold,
      color = Color.White.copy(alpha = alpha),
    )
  }
}

@Composable
fun ChatBubble(message: ChatMessage, modelName: String) {
  val isUser = message.role == ChatRole.User
  val corvusColors = CorvusTheme.colors
  val bubblePalette =
    remember(isUser, corvusColors) {
      ChatBubblePalette(
        background = if (isUser) corvusColors.userBubbleBackground else corvusColors.glassSurface,
        accent = if (isUser) corvusColors.glowPurple else corvusColors.glowCyan,
        shadow =
          if (isUser) corvusColors.glowPurple.copy(alpha = 0.2f)
          else corvusColors.glowCyan.copy(alpha = 0.2f),
      )
    }

  Row(
    modifier = Modifier.fillMaxWidth(),
    horizontalArrangement = if (isUser) Arrangement.End else Arrangement.Start,
  ) {
    if (!isUser) {
      AvatarWithGlow(corvusColors = corvusColors)
      Spacer(modifier = Modifier.width(8.dp))
    }

    ChatBubbleBody(
      isUser = isUser,
      modelName = modelName,
      message = message,
      contentColor = MaterialTheme.colorScheme.onSurface,
      corvusColors = corvusColors,
      bubblePalette = bubblePalette,
    )
  }
}

@Composable
private fun AvatarWithGlow(corvusColors: CorvusColorPalette) {
  val avatarGradient =
    remember(corvusColors.gradientPrimary) { Brush.linearGradient(corvusColors.gradientPrimary) }

  Box(
    modifier =
      Modifier.size(32.dp)
        .shadow(
          elevation = 4.dp,
          shape = CircleShape,
          spotColor = corvusColors.glowCyan.copy(alpha = 0.5f),
        )
        .background(brush = avatarGradient, shape = CircleShape),
    contentAlignment = Alignment.Center,
  ) {
    Text(
      text = "C",
      style = MaterialTheme.typography.labelMedium,
      fontWeight = FontWeight.Bold,
      color = Color.White,
    )
  }
}

@Composable
private fun ChatBubbleHeader(isUser: Boolean, modelName: String, corvusColors: CorvusColorPalette) {
  Text(
    text = if (isUser) "You" else modelName,
    style = MaterialTheme.typography.labelSmall,
    fontWeight = FontWeight.Medium,
    color = if (isUser) corvusColors.glowPurple else corvusColors.glowCyan,
  )
}

@Composable
private fun ChatBubbleBody(
  isUser: Boolean,
  modelName: String,
  message: ChatMessage,
  contentColor: Color,
  corvusColors: CorvusColorPalette,
  bubblePalette: ChatBubblePalette,
) {
  Box(
    modifier =
      Modifier.widthIn(max = 280.dp)
        .shadow(
          elevation = if (isUser) 6.dp else 4.dp,
          shape = ChatBubbleShape,
          spotColor = bubblePalette.shadow,
        )
  ) {
    Surface(
      shape = ChatBubbleShape,
      color = bubblePalette.background,
      border =
        BorderStroke(
          width = 1.dp,
          brush =
            Brush.horizontalGradient(
              listOf(bubblePalette.accent.copy(alpha = 0.3f), Color.Transparent)
            ),
        ),
    ) {
      Column(modifier = Modifier.padding(horizontal = 14.dp, vertical = 10.dp)) {
        ChatBubbleHeader(isUser = isUser, modelName = modelName, corvusColors = corvusColors)
        Spacer(modifier = Modifier.height(4.dp))
        Text(
          text = message.content,
          style = MaterialTheme.typography.bodyMedium,
          color = contentColor,
        )
      }
    }
  }
}

@Composable
fun StatusIndicator(active: Boolean, label: String, modifier: Modifier = Modifier) {
  val corvusColors = CorvusTheme.colors
  val color by
    animateColorAsState(
      targetValue = if (active) corvusColors.connected else corvusColors.disconnected,
      animationSpec = tween(300),
      label = "statusColor",
    )
  val glowAlpha by
    animateFloatAsState(
      targetValue = if (active) 0.6f else 0.2f,
      animationSpec = tween(300),
      label = "glowAlpha",
    )

  Row(modifier = modifier, verticalAlignment = Alignment.CenterVertically) {
    Box(
      modifier =
        Modifier.size(10.dp)
          .shadow(elevation = 4.dp, shape = CircleShape, spotColor = color.copy(alpha = glowAlpha))
          .background(color, CircleShape)
    )
    Spacer(modifier = Modifier.width(8.dp))
    Text(
      text = label,
      style = MaterialTheme.typography.labelSmall,
      color = color,
      fontWeight = FontWeight.Medium,
    )
  }
}

@Composable
fun ChatHeader(
  modelName: String,
  bridgeState: MobileBridgeUiState,
  showConfig: Boolean,
  onToggleConfig: () -> Unit,
) {
  val corvusColors = CorvusTheme.colors

  Row(
    modifier = Modifier.fillMaxWidth(),
    horizontalArrangement = Arrangement.SpaceBetween,
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Column {
      Text(
        text = modelName,
        style = MaterialTheme.typography.headlineSmall,
        fontWeight = FontWeight.Bold,
        color = MaterialTheme.colorScheme.onBackground,
      )
      Text(
        text =
          if (showConfig) bridgeStateHeadline(bridgeState) else bridgeStateDescription(bridgeState),
        style = MaterialTheme.typography.labelMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
      )
    }

    IconButton(
      onClick = onToggleConfig,
      modifier =
        Modifier.size(44.dp)
          .shadow(4.dp, CircleShape)
          .background(
            brush = Brush.linearGradient(corvusColors.gradientPrimary),
            shape = CircleShape,
          ),
    ) {
      Icon(
        imageVector = Icons.Default.Settings,
        contentDescription = SETTINGS_CONTENT_DESCRIPTION,
        tint = Color.White,
        modifier = Modifier.size(22.dp),
      )
    }
  }
}

@Composable
fun BridgeStatusCard(bridgeState: MobileBridgeUiState, modifier: Modifier = Modifier) {
  val description = bridgeStateDescription(bridgeState)
  val recovery = bridgeStateRecovery(bridgeState)
  val details = buildList {
    add(description)
    if (description != recovery) {
      add(recovery)
    }
  }

  diagnosticsCard(
    title = onboardingStateLabel(bridgeState.onboardingState),
    subtitle = bridgeStateHeadline(bridgeState),
    details = details,
    modifier = modifier,
  )
}

@Composable
fun SessionSelectionCard(
  sessions: List<RuntimeSession>,
  activeSessionId: String?,
  onResumeSession: (String) -> Unit,
  modifier: Modifier = Modifier,
) {
  if (sessions.isEmpty()) return

  diagnosticsCard(
    title = "Resumable sessions",
    subtitle = activeSessionId ?: "Select a session to resume",
    details = sessions.map { session -> session.title ?: session.id.value },
    modifier = modifier,
  )

  Column(
    modifier = modifier.padding(top = 12.dp),
    verticalArrangement = Arrangement.spacedBy(8.dp),
  ) {
    sessions.forEach { session ->
      OutlinedButton(
        onClick = { onResumeSession(session.id.value) },
        modifier = Modifier.fillMaxWidth(),
        shape = RoundedCornerShape(12.dp),
        colors = ButtonDefaults.outlinedButtonColors(contentColor = CorvusTheme.colors.glowCyan),
      ) {
        Text(text = session.title ?: session.id.value, fontWeight = FontWeight.Medium)
      }
    }
  }
}

@Composable
fun ApprovalCard(
  request: RuntimeApprovalRequest,
  onApprove: () -> Unit,
  onDeny: () -> Unit,
  modifier: Modifier = Modifier,
) {
  val corvusColors = CorvusTheme.colors
  Surface(
    modifier = modifier,
    shape = RoundedCornerShape(16.dp),
    color = corvusColors.glassSurface,
    border = BorderStroke(1.dp, corvusColors.glassOverlay),
  ) {
    Column(
      modifier = Modifier.fillMaxWidth().padding(16.dp),
      verticalArrangement = Arrangement.spacedBy(12.dp),
    ) {
      Text(
        text = "Approval required",
        style = MaterialTheme.typography.titleSmall,
        fontWeight = FontWeight.SemiBold,
        color = corvusColors.glowCyan,
      )
      Text(
        text = "${request.toolLabel}: ${request.reason}",
        style = MaterialTheme.typography.bodyMedium,
        color = MaterialTheme.colorScheme.onSurface,
      )
      Row(horizontalArrangement = Arrangement.spacedBy(12.dp), modifier = Modifier.fillMaxWidth()) {
        OutlinedButton(
          onClick = onDeny,
          modifier = Modifier.weight(1f),
          shape = RoundedCornerShape(12.dp),
        ) {
          Text("Deny")
        }
        GradientButton(text = "Approve", onClick = onApprove, modifier = Modifier.weight(1f))
      }
    }
  }
}

@Composable
fun diagnosticsCard(
  title: String,
  subtitle: String,
  details: List<String>,
  modifier: Modifier = Modifier,
) {
  val colors = MaterialTheme.colorScheme
  val corvusColors = CorvusTheme.colors

  Box(
    modifier =
      modifier
        .fillMaxWidth()
        .shadow(
          elevation = 4.dp,
          shape = DiagnosticsCardShape,
          spotColor = corvusColors.glowPurple.copy(alpha = 0.1f),
        )
  ) {
    Surface(
      shape = DiagnosticsCardShape,
      color = corvusColors.glassSurface,
      border = BorderStroke(1.dp, corvusColors.glassOverlay),
    ) {
      Column(
        modifier = Modifier.fillMaxWidth().padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
      ) {
        Box(
          modifier =
            Modifier.width(40.dp)
              .height(3.dp)
              .background(
                brush = Brush.horizontalGradient(corvusColors.gradientPrimary),
                shape = RoundedCornerShape(2.dp),
              )
        )

        Text(
          text = title,
          style = MaterialTheme.typography.titleSmall,
          fontWeight = FontWeight.SemiBold,
          color = corvusColors.glowCyan,
        )
        Text(
          text = subtitle,
          style = MaterialTheme.typography.bodySmall,
          color = colors.onSurfaceVariant,
        )
        details.forEach { line ->
          Text(
            text = "- $line",
            style = MaterialTheme.typography.bodySmall,
            color = colors.onSurface.copy(alpha = 0.7f),
          )
        }
      }
    }
  }
}

internal fun onboardingStateLabel(onboardingState: MobileOnboardingState): String =
  when (onboardingState.status) {
    MobileOnboardingStatus.TARGET_SELECTED -> TARGET_SELECTED_LABEL
    MobileOnboardingStatus.RECOVERY -> RECOVERY_REQUIRED_LABEL
    MobileOnboardingStatus.RUNTIME_PATH_CONFIRMED -> "Runtime available"
    MobileOnboardingStatus.TRUST_PENDING -> "Trust this surface"
    MobileOnboardingStatus.TRANSPORT_CONNECTING -> "Connect to runtime"
    MobileOnboardingStatus.SESSION_PENDING -> "Session pending"
    MobileOnboardingStatus.SESSION_READY -> "Session ready"
    MobileOnboardingStatus.BLOCKED -> RECOVERY_REQUIRED_LABEL
  }

internal fun bridgeStateHeadline(bridgeState: MobileBridgeUiState): String =
  when (bridgeState.onboardingState.status) {
    MobileOnboardingStatus.TARGET_SELECTED -> TARGET_SELECTED_LABEL
    MobileOnboardingStatus.RECOVERY -> RECOVERY_REQUIRED_LABEL
    // Client-first: Connect to an existing runtime, not local bridge
    MobileOnboardingStatus.TRUST_PENDING -> "Connect to runtime endpoint"
    MobileOnboardingStatus.SESSION_PENDING -> "Runtime connected. Start or resume a session next."
    MobileOnboardingStatus.SESSION_READY -> "Session ready on ${bridgeState.platformName}"
    // Client-first: Don't use "bridge" terminology
    MobileOnboardingStatus.BLOCKED -> "Connection attention needed on ${bridgeState.platformName}"
    MobileOnboardingStatus.RUNTIME_PATH_CONFIRMED -> "Runtime endpoint configured"
    MobileOnboardingStatus.TRANSPORT_CONNECTING -> "Connecting to runtime"
  }

internal fun bridgeStateDescription(bridgeState: MobileBridgeUiState): String =
  when (bridgeState.onboardingState.status) {
    // Client-first: Connect to an existing runtime, not "linking" mobile
    MobileOnboardingStatus.TRUST_PENDING ->
      "Configure connection to an existing runtime before starting a session."
    MobileOnboardingStatus.SESSION_PENDING ->
      "The runtime endpoint is reachable. Create or resume a chat session to continue."
    // Client-first: Describe as runtime connection, not CLI bridge
    MobileOnboardingStatus.SESSION_READY ->
      "Messages now flow through the configured runtime connection."
    MobileOnboardingStatus.BLOCKED -> bridgeStateRecovery(bridgeState)
    else -> "Corvus keeps connection, transport, and session steps separate."
  }

internal fun bridgeStateRecovery(bridgeState: MobileBridgeUiState): String =
  when (bridgeState.onboardingState.recoveryKind) {
    // Client-first: Guide user to configure target/endpoint
    MobileRecoveryKind.NO_TARGET_CONFIGURED ->
      "Select a connection target first. Choose endpoint URL, trusted companion, or another supported method."
    MobileRecoveryKind.TARGET_NOT_REACHABLE ->
      "The selected target is not reachable. Check your network connection and try again."
    MobileRecoveryKind.TRUST_NOT_ESTABLISHED ->
      "Trust has not been established. Complete the pairing or companion authentication first."
    // Client-first: Don't mention CLI, describe as runtime connection
    MobileRecoveryKind.RUNTIME_UNAVAILABLE ->
      "Make sure the runtime is running and reachable, then retry."
    MobileRecoveryKind.TRANSPORT_UNAVAILABLE ->
      "The connection transport is unavailable. Retry first, or reconfigure the connection target."
    MobileRecoveryKind.LINKED_BUT_NOT_SESSION_READY ->
      "The runtime is connected, but session operations are unavailable. Retry the connection check."
    MobileRecoveryKind.SESSION_UNAVAILABLE ->
      "Your last session is no longer resumable. Start a new session or choose another available session."
    MobileRecoveryKind.ENVIRONMENT_UNSUPPORTED ->
      "This environment requires a supported connection method. Check your platform's available options."
    null ->
      when (bridgeState.onboardingState.status) {
        MobileOnboardingStatus.TARGET_SELECTED ->
          "A target is selected but not yet configured. Complete the connection setup."
        MobileOnboardingStatus.RECOVERY -> "Recovery is needed. Follow the on-screen instructions."
        // Client-first: Don't mention "linking"
        MobileOnboardingStatus.TRUST_PENDING ->
          "Configure the connection to your runtime endpoint first."
        MobileOnboardingStatus.SESSION_PENDING ->
          "Start a session after the runtime connection is confirmed reachable."
        else -> "No recovery action needed."
      }
  }
