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
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
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
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import com.profiletailors.corvus.ui.theme.CorvusColorPalette
import com.profiletailors.corvus.ui.theme.CorvusTheme

internal val ChatPanelShape = RoundedCornerShape(20.dp)
internal val ConfigPanelShape = RoundedCornerShape(20.dp)
internal val DiagnosticsCardShape = RoundedCornerShape(16.dp)
internal val ChatBubbleShape = RoundedCornerShape(18.dp)

@Immutable data class ChatMessage(val id: Int, val role: ChatRole, val content: String)

enum class ChatRole {
  User,
  Assistant,
}

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
    )
  }
}

@Composable
private fun AvatarWithGlow(corvusColors: CorvusColorPalette) {
  Box(
    modifier =
      Modifier.size(32.dp)
        .shadow(
          elevation = 4.dp,
          shape = CircleShape,
          spotColor = corvusColors.glowCyan.copy(alpha = 0.5f),
        )
        .background(
          brush = Brush.linearGradient(corvusColors.gradientPrimary),
          shape = CircleShape,
        ),
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
) {
  Box(
    modifier =
      Modifier.widthIn(max = 280.dp)
        .shadow(
          elevation = if (isUser) 6.dp else 4.dp,
          shape = ChatBubbleShape,
          spotColor =
            if (isUser) corvusColors.glowPurple.copy(alpha = 0.2f)
            else corvusColors.glowCyan.copy(alpha = 0.2f),
        )
  ) {
    Surface(
      shape = ChatBubbleShape,
      color = if (isUser) corvusColors.userBubbleBackground else corvusColors.glassSurface,
      border =
        BorderStroke(
          width = 1.dp,
          brush =
            Brush.horizontalGradient(
              listOf(
                if (isUser) corvusColors.glowPurple.copy(alpha = 0.3f)
                else corvusColors.glowCyan.copy(alpha = 0.3f),
                Color.Transparent,
              )
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
fun ChatInputField(
  value: String,
  onValueChange: (String) -> Unit,
  onSend: () -> Unit,
  placeholder: String,
  modifier: Modifier = Modifier,
  enabled: Boolean = true,
) {
  val colors = MaterialTheme.colorScheme
  val corvusColors = CorvusTheme.colors
  val gradient =
    remember(corvusColors.gradientPrimary) {
      Brush.horizontalGradient(corvusColors.gradientPrimary)
    }
  val isEnabled = enabled && value.trim().isNotBlank()

  Row(modifier = modifier.fillMaxWidth(), verticalAlignment = Alignment.Bottom) {
    Surface(
      modifier = Modifier.weight(1f),
      shape = RoundedCornerShape(16.dp),
      color = corvusColors.glassSurface,
      border = BorderStroke(1.dp, corvusColors.glassOverlay),
    ) {
      OutlinedTextField(
        value = value,
        onValueChange = onValueChange,
        modifier = Modifier.fillMaxWidth(),
        enabled = enabled,
        placeholder = {
          Text(text = placeholder, color = colors.onSurfaceVariant.copy(alpha = 0.6f))
        },
        colors =
          OutlinedTextFieldDefaults.colors(
            focusedBorderColor = Color.Transparent,
            unfocusedBorderColor = Color.Transparent,
            focusedTextColor = colors.onSurface,
            unfocusedTextColor = colors.onSurface,
            disabledBorderColor = Color.Transparent,
            disabledTextColor = colors.onSurfaceVariant,
          ),
        maxLines = 4,
        textStyle = TextStyle(fontFamily = FontFamily.SansSerif),
      )
    }

    Spacer(modifier = Modifier.width(12.dp))

    Box(
      modifier =
        Modifier.size(48.dp)
          .shadow(
            elevation = 6.dp,
            shape = CircleShape,
            spotColor = if (isEnabled) corvusColors.glowPurple else Color.Gray,
          )
          .clip(CircleShape)
          .background(
            if (isEnabled) gradient else Brush.linearGradient(listOf(Color.Gray, Color.Gray))
          ),
      contentAlignment = Alignment.Center,
    ) {
      IconButton(onClick = onSend, enabled = isEnabled) {
        Icon(
          imageVector = Icons.AutoMirrored.Filled.Send,
          contentDescription = "Send",
          tint = Color.White,
          modifier = Modifier.size(22.dp),
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
        contentDescription = "Settings",
        tint = Color.White,
        modifier = Modifier.size(22.dp),
      )
    }
  }
}

@Composable
fun BridgeStatusCard(
  bridgeState: MobileBridgeUiState,
  actions: ChatWorkspaceActions,
  modifier: Modifier = Modifier,
) {
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
    MobileOnboardingStatus.RUNTIME_PATH_CONFIRMED -> "Runtime available"
    MobileOnboardingStatus.TRUST_PENDING -> "Trust this surface"
    MobileOnboardingStatus.TRANSPORT_CONNECTING -> "Connect to runtime"
    MobileOnboardingStatus.SESSION_PENDING -> "Session pending"
    MobileOnboardingStatus.SESSION_READY -> "Session ready"
    MobileOnboardingStatus.BLOCKED -> "Recovery required"
  }

internal fun bridgeStateHeadline(bridgeState: MobileBridgeUiState): String =
  when (bridgeState.onboardingState.status) {
    MobileOnboardingStatus.TRUST_PENDING -> "Link this app to local Corvus"
    MobileOnboardingStatus.SESSION_PENDING -> "Bridge linked. Start or resume a session next."
    MobileOnboardingStatus.SESSION_READY -> "Linked session ready on ${bridgeState.platformName}"
    MobileOnboardingStatus.BLOCKED -> "Bridge attention needed on ${bridgeState.platformName}"
    MobileOnboardingStatus.RUNTIME_PATH_CONFIRMED -> "Runtime path confirmed"
    MobileOnboardingStatus.TRANSPORT_CONNECTING -> "Connecting to runtime"
  }

internal fun bridgeStateDescription(bridgeState: MobileBridgeUiState): String =
  when (bridgeState.onboardingState.status) {
    MobileOnboardingStatus.TRUST_PENDING ->
      "Complete mobile linking before Corvus can trust this surface."
    MobileOnboardingStatus.SESSION_PENDING ->
      "The bridge can reach Corvus. Create or resume a chat session to continue."
    MobileOnboardingStatus.SESSION_READY ->
      "Messages now flow through the local CLI bridge instead of HTTP pairing."
    MobileOnboardingStatus.BLOCKED -> bridgeStateRecovery(bridgeState)
    else -> "Corvus keeps mobile trust, transport, and session steps separate."
  }

internal fun bridgeStateRecovery(bridgeState: MobileBridgeUiState): String =
  when (bridgeState.onboardingState.recoveryKind) {
    MobileRecoveryKind.RUNTIME_UNAVAILABLE ->
      "Make sure the Corvus CLI or approved companion path is running, then retry."
    MobileRecoveryKind.LINKED_BUT_NOT_SESSION_READY ->
      "The app is linked, but session operations are still unavailable. Retry the bridge check."
    MobileRecoveryKind.ENVIRONMENT_UNSUPPORTED ->
      "This environment needs the approved companion path. Do not switch to HTTP pairing here."
    null ->
      when (bridgeState.onboardingState.status) {
        MobileOnboardingStatus.TRUST_PENDING ->
          "Linking is the trust step for mobile. Pairing codes and bearer tokens stay on web surfaces."
        MobileOnboardingStatus.SESSION_PENDING ->
          "Start a session after the bridge confirms Corvus is reachable and session-capable."
        else -> "No recovery action needed."
      }
  }

internal fun buildLocalAssistantReply(
  prompt: String,
  modelName: String,
  bridgeState: MobileBridgeUiState,
): String {
  val sessionLabel = bridgeState.snapshot.sessionId ?: "pending-session"
  return "[$modelName] Received \"$prompt\" through the local bridge for session $sessionLabel."
}
