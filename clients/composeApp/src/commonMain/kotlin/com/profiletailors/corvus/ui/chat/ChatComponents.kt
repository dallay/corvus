package com.profiletailors.corvus.ui.chat

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
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
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.StrokeCap
import androidx.compose.ui.graphics.drawscope.Stroke
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
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
private const val SESSION_HISTORY_CONTENT_DESCRIPTION = "Session History"
private const val COMPONENT_ANIMATION_DURATION_MS = 300
private const val DISABLED_BUTTON_ALPHA = 0.5f
private const val BUTTON_SHADOW_ALPHA = 0.3f
private const val ACTIVE_STATUS_GLOW_ALPHA = 0.6f
private const val INACTIVE_STATUS_GLOW_ALPHA = 0.2f
private const val SESSION_ID_TRUNCATE_LENGTH = 8

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
  showSessionHistory: Boolean,
  onToggleConfig: () -> Unit,
  onToggleSessionHistory: () -> Unit,
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

    Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
      if (bridgeState.isChatReady) {
        SessionHistoryToggleButton(
          showSessionHistory = showSessionHistory,
          iconBackgroundBrush = iconBackgroundBrush,
          corvusColors = corvusColors,
          onToggle = onToggleSessionHistory,
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
}

@Composable
private fun SessionHistoryToggleButton(
  showSessionHistory: Boolean,
  iconBackgroundBrush: Brush,
  corvusColors: CorvusColorPalette,
  onToggle: () -> Unit,
) {
  val activeBackground =
    remember(corvusColors.glowCyan) {
      Brush.linearGradient(
        listOf(
          corvusColors.glowCyan.copy(alpha = 0.4f),
          corvusColors.glowCyan.copy(alpha = 0.2f),
        )
      )
    }
  val background = if (showSessionHistory) activeBackground else iconBackgroundBrush
  IconButton(
    onClick = onToggle,
    modifier =
      Modifier.size(44.dp)
        .shadow(4.dp, CircleShape)
        .background(brush = background, shape = CircleShape),
  ) {
    HistoryIcon(
      tint = Color.White,
      contentDescription = SESSION_HISTORY_CONTENT_DESCRIPTION,
      modifier = Modifier.size(22.dp),
    )
  }
}

@Composable
private fun HistoryIcon(tint: Color, contentDescription: String?, modifier: Modifier = Modifier) {
  val semanticsModifier =
    if (contentDescription != null) {
      modifier.then(
        Modifier.semantics { this.contentDescription = contentDescription }
      )
    } else {
      modifier
    }
  Canvas(modifier = semanticsModifier) {
    val cx = size.width / 2f
    val cy = size.height / 2f
    val r = size.minDimension / 2f * 0.85f
    val strokeWidth = size.minDimension * 0.1f
    drawCircle(
      color = tint,
      radius = r,
      center = Offset(cx, cy),
      style = Stroke(width = strokeWidth),
    )
    val handLengthClock = r * 0.55f
    val handLengthMinute = r * 0.7f
    drawLine(
      color = tint,
      start = Offset(cx, cy),
      end = Offset(cx, cy - handLengthClock),
      strokeWidth = strokeWidth,
      cap = StrokeCap.Round,
    )
    drawLine(
      color = tint,
      start = Offset(cx, cy),
      end = Offset(cx + handLengthMinute, cy),
      strokeWidth = strokeWidth,
      cap = StrokeCap.Round,
    )
  }
}

@Suppress("FunctionNaming") // Composable functions follow PascalCase per Compose conventions
@Composable
fun SessionHistoryPanel(
  sessions: List<RuntimeSession>,
  activeSessionId: String?,
  onSwitchSession: (String) -> Unit,
  onNewSession: () -> Unit,
  modifier: Modifier = Modifier,
) {
  val corvusColors = CorvusTheme.colors

  Surface(
    modifier = modifier.fillMaxSize(),
    shape = RoundedCornerShape(20.dp),
    color = corvusColors.glassSurface,
  ) {
    val backgroundBrush = remember {
      Brush.verticalGradient(listOf(Color.White.copy(alpha = 0.05f), Color.Transparent))
    }
    Box(modifier = Modifier.background(brush = backgroundBrush)) {
      LazyColumn(
        modifier = Modifier.fillMaxSize().padding(20.dp),
        verticalArrangement = Arrangement.spacedBy(12.dp),
      ) {
        item {
          Row(
            modifier = Modifier.fillMaxWidth(),
            horizontalArrangement = Arrangement.SpaceBetween,
            verticalAlignment = Alignment.CenterVertically,
          ) {
            Text(
              text = "Session History",
              style = MaterialTheme.typography.titleLarge,
              fontWeight = FontWeight.Bold,
              color = MaterialTheme.colorScheme.onSurface,
            )
            GradientButton(
              text = "New Session",
              onClick = onNewSession,
            )
          }
        }

        if (sessions.isEmpty()) {
          item {
            Text(
              text = "No past sessions. Start a new session to begin.",
              style = MaterialTheme.typography.bodyMedium,
              color = MaterialTheme.colorScheme.onSurfaceVariant,
              modifier = Modifier.padding(top = 8.dp),
            )
          }
        } else {
          itemsIndexed(items = sessions, key = { _, s -> s.id.value }) { _, session ->
            SessionHistoryItem(
              session = session,
              isActive = session.id.value == activeSessionId,
              onSwitch = { onSwitchSession(session.id.value) },
            )
          }
        }

        item { Spacer(modifier = Modifier.height(32.dp)) }
      }
    }
  }
}

@Composable
private fun SessionHistoryItem(
  session: RuntimeSession,
  isActive: Boolean,
  onSwitch: () -> Unit,
) {
  val corvusColors = CorvusTheme.colors
  val borderColor =
    if (isActive) corvusColors.glowCyan.copy(alpha = 0.6f)
    else corvusColors.glassOverlay

  Surface(
    shape = RoundedCornerShape(14.dp),
    color = if (isActive) corvusColors.glowCyan.copy(alpha = 0.08f) else corvusColors.glassSurface,
    border = BorderStroke(1.dp, borderColor),
  ) {
    Row(
      modifier =
        Modifier.fillMaxWidth()
          .clickable(onClick = onSwitch)
          .padding(horizontal = 16.dp, vertical = 12.dp),
      horizontalArrangement = Arrangement.SpaceBetween,
      verticalAlignment = Alignment.CenterVertically,
    ) {
      Column(modifier = Modifier.weight(1f)) {
        Text(
          text = session.title ?: truncateSessionId(session.id.value),
          style = MaterialTheme.typography.bodyMedium,
          fontWeight = if (isActive) FontWeight.SemiBold else FontWeight.Normal,
          color =
            if (isActive) corvusColors.glowCyan else MaterialTheme.colorScheme.onSurface,
        )
        if (isActive) {
          Text(
            text = "Active",
            style = MaterialTheme.typography.labelSmall,
            color = corvusColors.glowCyan.copy(alpha = 0.8f),
          )
        }
      }

      if (!isActive) {
        OutlinedButton(
          onClick = onSwitch,
          shape = RoundedCornerShape(10.dp),
          colors = ButtonDefaults.outlinedButtonColors(contentColor = corvusColors.glowCyan),
        ) {
          Text(text = "Open", style = MaterialTheme.typography.labelMedium)
        }
      }
    }
  }
}

internal fun truncateSessionId(sessionId: String): String =
  if (sessionId.length > SESSION_ID_TRUNCATE_LENGTH) {
    sessionId.take(SESSION_ID_TRUNCATE_LENGTH) + "…"
  } else {
    sessionId
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
