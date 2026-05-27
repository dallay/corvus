@file:Suppress("MatchingDeclarationName") // File contains multiple related Composable declarations

package com.profiletailors.corvus.ui.chat

import androidx.compose.foundation.BorderStroke
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.filled.Send
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.OutlinedTextFieldDefaults
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.SolidColor
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import com.profiletailors.corvus.ui.theme.CorvusTheme

@Immutable
data class ChatInputFieldProps(
  val value: String,
  val onValueChange: (String) -> Unit,
  val onSend: (String) -> Unit,
  val placeholder: String,
  val enabled: Boolean = true,
)

@Suppress("FunctionNaming") // Composable functions follow PascalCase per Compose conventions
@Composable
fun ChatInputField(props: ChatInputFieldProps, modifier: Modifier = Modifier) {
  val colors = MaterialTheme.colorScheme
  val corvusColors = CorvusTheme.colors
  val gradient =
    remember(corvusColors.gradientPrimary) {
      Brush.horizontalGradient(corvusColors.gradientPrimary)
    }
  val isEnabled = props.enabled && props.value.trim().isNotBlank()

  Row(modifier = modifier.fillMaxWidth(), verticalAlignment = Alignment.Bottom) {
    Surface(
      modifier = Modifier.weight(1f),
      shape = RoundedCornerShape(16.dp),
      color = corvusColors.glassSurface,
      border = BorderStroke(1.dp, corvusColors.glassOverlay),
    ) {
      OutlinedTextField(
        value = props.value,
        onValueChange = props.onValueChange,
        modifier = Modifier.fillMaxWidth(),
        enabled = props.enabled,
        placeholder = {
          val placeholderColor =
            remember(colors.onSurfaceVariant) { colors.onSurfaceVariant.copy(alpha = 0.6f) }
          Text(text = props.placeholder, color = placeholderColor)
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

    SendButton(
      isEnabled = isEnabled,
      gradient = gradient,
      glowColor = corvusColors.glowPurple,
      onSend = { props.onSend(props.value) },
    )
  }
}

@Suppress("FunctionNaming") // Composable functions follow PascalCase per Compose conventions
@Composable
private fun SendButton(isEnabled: Boolean, gradient: Brush, glowColor: Color, onSend: () -> Unit) {
  val sendButtonModifier =
    remember(isEnabled, glowColor, gradient) {
      Modifier.size(48.dp)
        .shadow(
          elevation = 6.dp,
          shape = CircleShape,
          spotColor = if (isEnabled) glowColor else Color.Gray,
        )
        .clip(CircleShape)
        .background(if (isEnabled) gradient else SolidColor(Color.Gray))
    }

  Box(modifier = sendButtonModifier, contentAlignment = Alignment.Center) {
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
