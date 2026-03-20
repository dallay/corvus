package com.profiletailors.corvus.ui.chat

import androidx.compose.animation.animateColorAsState
import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
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
import androidx.compose.material.icons.filled.Send
import androidx.compose.material.icons.filled.Settings
import androidx.compose.material.icons.filled.Visibility
import androidx.compose.material.icons.filled.VisibilityOff
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
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.input.VisualTransformation
import androidx.compose.ui.unit.dp
import com.profiletailors.corvus.ui.theme.CorvusColors
import com.profiletailors.corvus.ui.theme.GradientPurpleCyan

// ============================================================================
// Corvus Chat - Futuristic Tech UI Components
// ============================================================================

// --- Shapes (Futuristic Rounded) ---
internal val ChatPanelShape = RoundedCornerShape(20.dp)
internal val ConfigPanelShape = RoundedCornerShape(20.dp)
internal val EndpointCardShape = RoundedCornerShape(16.dp)
internal val ChatBubbleShape = RoundedCornerShape(18.dp)

// --- Chat Message Types ---
@Immutable data class ChatMessage(val id: Int, val role: ChatRole, val content: String)

enum class ChatRole {
  User,
  Assistant,
}

// ============================================================================
// Glassmorphism Surface
// ============================================================================

@Composable
fun GlassSurface(modifier: Modifier = Modifier, content: @Composable () -> Unit) {
  Surface(
    modifier = modifier,
    shape = RoundedCornerShape(20.dp),
    color = CorvusColors.glassSurface,
    tonalElevation = 0.dp,
  ) {
    Box(
      modifier =
        Modifier.background(
          brush =
            Brush.verticalGradient(
              colors = listOf(Color.White.copy(alpha = 0.1f), Color.Transparent)
            )
        )
    ) {
      content()
    }
  }
}

// ============================================================================
// Gradient Button
// ============================================================================

@Composable
fun GradientButton(
  text: String,
  onClick: () -> Unit,
  modifier: Modifier = Modifier,
  enabled: Boolean = true,
) {
  val gradient = remember { Brush.horizontalGradient(GradientPurpleCyan) }
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
          spotColor = CorvusColors.glowPurple.copy(alpha = 0.3f),
        )
        .clip(RoundedCornerShape(14.dp))
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

// ============================================================================
// Chat Bubble - Futuristic Style
// ============================================================================

@Composable
fun ChatBubble(message: ChatMessage, modelName: String) {
  val isUser = message.role == ChatRole.User
  val colors = MaterialTheme.colorScheme

  // Gradient colors based on role
  val gradientColors =
    if (isUser) {
      listOf(CorvusColors.userBubble, CorvusColors.userBubbleBackground)
    } else {
      listOf(CorvusColors.aiBubble, CorvusColors.aiBubbleBackground)
    }

  Row(
    modifier = Modifier.fillMaxWidth(),
    horizontalArrangement = if (isUser) Arrangement.End else Arrangement.Start,
  ) {
    if (!isUser) {
      // AI Avatar with glow
      Box(
        modifier =
          Modifier.size(32.dp)
            .shadow(
              elevation = 4.dp,
              shape = CircleShape,
              spotColor = CorvusColors.glowCyan.copy(alpha = 0.5f),
            )
            .background(brush = Brush.linearGradient(GradientPurpleCyan), shape = CircleShape),
        contentAlignment = Alignment.Center,
      ) {
        Text(
          text = "C",
          style = MaterialTheme.typography.labelMedium,
          fontWeight = FontWeight.Bold,
          color = Color.White,
        )
      }
      Spacer(modifier = Modifier.width(8.dp))
    }

    Box(
      modifier =
        Modifier.widthIn(max = 280.dp)
          .shadow(
            elevation = if (isUser) 6.dp else 4.dp,
            shape = ChatBubbleShape,
            spotColor =
              if (isUser) CorvusColors.glowPurple.copy(alpha = 0.2f)
              else CorvusColors.glowCyan.copy(alpha = 0.2f),
          )
    ) {
      Surface(
        shape = ChatBubbleShape,
        color =
          if (isUser) {
            CorvusColors.glowPurple.copy(alpha = 0.15f)
          } else {
            CorvusColors.glassSurface
          },
        border =
          BorderStroke(
            width = 1.dp,
            brush =
              Brush.horizontalGradient(
                colors =
                  listOf(
                    if (isUser) CorvusColors.glowPurple.copy(alpha = 0.3f)
                    else CorvusColors.glowCyan.copy(alpha = 0.3f),
                    Color.Transparent,
                  )
              ),
          ),
      ) {
        Column(modifier = Modifier.padding(horizontal = 14.dp, vertical = 10.dp)) {
          Text(
            text = if (isUser) "You" else modelName,
            style = MaterialTheme.typography.labelSmall,
            fontWeight = FontWeight.Medium,
            color = if (isUser) CorvusColors.glowPurple else CorvusColors.glowCyan,
          )
          Spacer(modifier = Modifier.height(4.dp))
          Text(
            text = message.content,
            style = MaterialTheme.typography.bodyMedium,
            color = colors.onSurface,
          )
        }
      }
    }
  }
}

// ============================================================================
// Chat Input Field - Futuristic Style
// ============================================================================

@Composable
fun ChatInputField(
  value: String,
  onValueChange: (String) -> Unit,
  onSend: () -> Unit,
  placeholder: String,
  modifier: Modifier = Modifier,
) {
  val colors = MaterialTheme.colorScheme
  val gradient = remember { Brush.horizontalGradient(GradientPurpleCyan) }
  val isEnabled = value.trim().isNotBlank()

  Row(modifier = modifier.fillMaxWidth(), verticalAlignment = Alignment.Bottom) {
    // Glassmorphism Input
    Surface(
      modifier = Modifier.weight(1f),
      shape = RoundedCornerShape(16.dp),
      color = CorvusColors.glassSurface,
      border = BorderStroke(1.dp, CorvusColors.glassOverlay),
    ) {
      OutlinedTextField(
        value = value,
        onValueChange = onValueChange,
        modifier = Modifier.fillMaxWidth(),
        placeholder = {
          Text(text = placeholder, color = colors.onSurfaceVariant.copy(alpha = 0.6f))
        },
        colors =
          OutlinedTextFieldDefaults.colors(
            focusedBorderColor = Color.Transparent,
            unfocusedBorderColor = Color.Transparent,
            focusedTextColor = colors.onSurface,
            unfocusedTextColor = colors.onSurface,
          ),
        maxLines = 4,
        textStyle = TextStyle(fontFamily = FontFamily.SansSerif),
      )
    }

    Spacer(modifier = Modifier.width(12.dp))

    // Gradient Send Button
    Box(
      modifier =
        Modifier.size(48.dp)
          .shadow(
            elevation = 6.dp,
            shape = CircleShape,
            spotColor = if (isEnabled) CorvusColors.glowPurple else Color.Gray,
          )
          .clip(CircleShape)
          .background(
            if (isEnabled) gradient else Brush.linearGradient(listOf(Color.Gray, Color.Gray))
          )
          .padding(0.dp),
      contentAlignment = Alignment.Center,
    ) {
      IconButton(onClick = onSend, enabled = isEnabled) {
        Icon(
          imageVector = Icons.Default.Send,
          contentDescription = "Send",
          tint = Color.White,
          modifier = Modifier.size(22.dp),
        )
      }
    }
  }
}

// ============================================================================
// Status Indicator - Futuristic Glow Dot
// ============================================================================

@Composable
fun StatusIndicator(connected: Boolean, modifier: Modifier = Modifier) {
  val color by
    animateColorAsState(
      targetValue = if (connected) CorvusColors.connected else CorvusColors.disconnected,
      animationSpec = tween(300),
      label = "statusColor",
    )

  val glowAlpha by
    animateFloatAsState(
      targetValue = if (connected) 0.6f else 0.2f,
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
      text = if (connected) "Connected" else "Disconnected",
      style = MaterialTheme.typography.labelSmall,
      color = color,
      fontWeight = FontWeight.Medium,
    )
  }
}

// ============================================================================
// Header - Futuristic Style
// ============================================================================

@Composable
fun ChatHeader(modelName: String, showConfig: Boolean, onToggleConfig: () -> Unit) {
  Row(
    modifier = Modifier.fillMaxWidth(),
    horizontalArrangement = Arrangement.SpaceBetween,
    verticalAlignment = Alignment.CenterVertically,
  ) {
    Column {
      // Gradient Title
      Text(
        text = modelName,
        style = MaterialTheme.typography.headlineSmall,
        fontWeight = FontWeight.Bold,
        color = MaterialTheme.colorScheme.onBackground,
      )
      Text(
        text = if (showConfig) "Gateway Configuration" else "Always-on AI Agent",
        style = MaterialTheme.typography.labelMedium,
        color = MaterialTheme.colorScheme.onSurfaceVariant,
      )
    }

    IconButton(
      onClick = onToggleConfig,
      modifier =
        Modifier.size(44.dp)
          .shadow(4.dp, CircleShape)
          .background(brush = Brush.linearGradient(GradientPurpleCyan), shape = CircleShape),
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

// ============================================================================
// Password Text Field - Futuristic Style
// ============================================================================

@Composable
fun passwordTextField(
  value: String,
  onValueChange: (String) -> Unit,
  label: String,
  placeholder: String,
  modifier: Modifier = Modifier,
) {
  var isVisible by remember { mutableStateOf(false) }
  val colors = MaterialTheme.colorScheme

  OutlinedTextField(
    value = value,
    onValueChange = onValueChange,
    label = { Text(label, color = colors.onSurfaceVariant) },
    placeholder = { Text(placeholder, color = colors.onSurfaceVariant.copy(alpha = 0.5f)) },
    singleLine = true,
    modifier = modifier.fillMaxWidth(),
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

// ============================================================================
// Endpoint Card - Futuristic Style
// ============================================================================

@Composable
fun endpointCard(title: String, subtitle: String, details: List<String>) {
  val colors = MaterialTheme.colorScheme

  Box(
    modifier =
      Modifier.fillMaxWidth()
        .shadow(
          elevation = 4.dp,
          shape = EndpointCardShape,
          spotColor = CorvusColors.glowPurple.copy(alpha = 0.1f),
        )
  ) {
    Surface(
      shape = EndpointCardShape,
      color = CorvusColors.glassSurface,
      border = BorderStroke(1.dp, CorvusColors.glassOverlay),
    ) {
      Column(
        modifier = Modifier.fillMaxWidth().padding(16.dp),
        verticalArrangement = Arrangement.spacedBy(8.dp),
      ) {
        // Gradient accent line
        Box(
          modifier =
            Modifier.width(40.dp)
              .height(3.dp)
              .background(
                brush = Brush.horizontalGradient(GradientPurpleCyan),
                shape = RoundedCornerShape(2.dp),
              )
        )

        Text(
          text = title,
          style = MaterialTheme.typography.titleSmall,
          fontWeight = FontWeight.SemiBold,
          color = CorvusColors.glowCyan,
        )
        Text(
          text = subtitle,
          style = MaterialTheme.typography.bodySmall,
          fontFamily = FontFamily.Monospace,
          color = colors.onSurfaceVariant,
        )
        details.forEach { line ->
          Text(
            text = "• $line",
            style = MaterialTheme.typography.bodySmall,
            color = colors.onSurface.copy(alpha = 0.7f),
          )
        }
      }
    }
  }
}

// ============================================================================
// Helper Functions
// ============================================================================

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
