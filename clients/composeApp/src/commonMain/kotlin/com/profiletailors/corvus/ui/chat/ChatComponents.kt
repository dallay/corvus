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
private const val COMPONENT_ANIMATION_DURATION_MS = 300
private const val DISABLED_BUTTON_ALPHA = 0.5f
private const val BUTTON_SHADOW_ALPHA = 0.3f
private const val ACTIVE_STATUS_GLOW_ALPHA = 0.6f
private const val INACTIVE_STATUS_GLOW_ALPHA = 0.2f

@Immutable data class ChatMessage(val id: Int, val role: ChatRole, val content: String)

enum class ChatRole {
  User,
  Assistant,
}

@Immutable
private data class ChatBubblePalette(
  val background: Color,
  val accent: Color,
  val shadow: Color,
  val content: Color,
  val title: Color,
)

@Composable
fun GlassSurface(modifier: Modifier = Modifier, content: @Composable () -> Unit) {
  val corvusColors = CorvusTheme.colors
  val backgroundBrush = remember {
    Brush.verticalGradient(listOf(Color.White.copy(alpha = 0.1f), Color.Transparent))
  }

  Surface(
    modifier = modifier,
    shape = RoundedCornerShape(20.dp),
    color = corvusColors.glassSurface,
    tonalElevation = 0.dp,
  ) {
    Box(modifier = Modifier.background(brush = backgroundBrush)) { content() }
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
      targetValue = if (enabled) 1f else DISABLED_BUTTON_ALPHA,
      animationSpec = tween(COMPONENT_ANIMATION_DURATION_MS),
      label = "buttonAlpha",
    )

  Box(
    modifier =
      modifier
        .shadow(
          elevation = 8.dp,
          shape = RoundedCornerShape(14.dp),
          spotColor = corvusColors.glowPurple.copy(alpha = BUTTON_SHADOW_ALPHA),
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
  val contentColor = MaterialTheme.colorScheme.onSurface
  val bubblePalette =
    remember(isUser, corvusColors, contentColor) {
      ChatBubblePalette(
        background = if (isUser) corvusColors.userBubbleBackground else corvusColors.glassSurface,
        accent = if (isUser) corvusColors.glowPurple else corvusColors.glowCyan,
        shadow =
          if (isUser) corvusColors.glowPurple.copy(alpha = 0.2f)
          else corvusColors.glowCyan.copy(alpha = 0.2f),
        content = contentColor,
        title = if (isUser) corvusColors.glowPurple else corvusColors.glowCyan,
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
private fun ChatBubbleHeader(isUser: Boolean, modelName: String, titleColor: Color) {
  Text(
    text = if (isUser) "You" else modelName,
    style = MaterialTheme.typography.labelSmall,
    fontWeight = FontWeight.Medium,
    color = titleColor,
  )
}

@Composable
private fun ChatBubbleBody(
  isUser: Boolean,
  modelName: String,
  message: ChatMessage,
  bubblePalette: ChatBubblePalette,
) {
  val borderBrush =
    remember(bubblePalette) {
      Brush.horizontalGradient(listOf(bubblePalette.accent.copy(alpha = 0.3f), Color.Transparent))
    }

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
      border = BorderStroke(width = 1.dp, brush = borderBrush),
    ) {
      Column(modifier = Modifier.padding(horizontal = 14.dp, vertical = 10.dp)) {
        ChatBubbleHeader(isUser = isUser, modelName = modelName, titleColor = bubblePalette.title)
        Spacer(modifier = Modifier.height(4.dp))
        Text(
          text = message.content,
          style = MaterialTheme.typography.bodyMedium,
          color = bubblePalette.content,
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
      animationSpec = tween(COMPONENT_ANIMATION_DURATION_MS),
      label = "statusColor",
    )
  val glowAlpha by
    animateFloatAsState(
      targetValue = if (active) ACTIVE_STATUS_GLOW_ALPHA else INACTIVE_STATUS_GLOW_ALPHA,
      animationSpec = tween(COMPONENT_ANIMATION_DURATION_MS),
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
  val iconBackgroundBrush =
    remember(corvusColors.gradientPrimary) { Brush.linearGradient(corvusColors.gradientPrimary) }

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
          .background(brush = iconBackgroundBrush, shape = CircleShape),
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

@Suppress("FunctionNaming") // Composable functions follow PascalCase per Compose conventions
@Composable
fun SessionSelectionCard(
  sessions: List<RuntimeSession>,
  activeSessionId: String?,
  onResumeSession: (String) -> Unit,
  modifier: Modifier = Modifier,
) {
  if (sessions.isEmpty()) return

  val sessionDetails =
    remember(sessions) { sessions.map { session -> session.title ?: session.id.value } }

  diagnosticsCard(
    title = "Resumable sessions",
    subtitle = activeSessionId ?: "Select a session to resume",
    details = sessionDetails,
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

@Suppress("FunctionNaming") // Composable functions follow PascalCase per Compose conventions
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
  val indicatorBrush =
    remember(corvusColors.gradientPrimary) {
      Brush.horizontalGradient(corvusColors.gradientPrimary)
    }

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
              .background(brush = indicatorBrush, shape = RoundedCornerShape(2.dp))
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
