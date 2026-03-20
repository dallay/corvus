package com.profiletailors.corvus.ui.theme

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.CompositionLocalProvider
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.TextStyle
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

// ============================================================================
// Corvus AI - Futuristic Tech Theme
// ============================================================================

private val DarkColorScheme =
  darkColorScheme(
    primary = Purple,
    onPrimary = Color.White,
    primaryContainer = PurpleDark,
    onPrimaryContainer = Color.White,
    secondary = Cyan,
    onSecondary = Color.White,
    secondaryContainer = CyanDark,
    onSecondaryContainer = Color.White,
    tertiary = PurpleLight,
    onTertiary = Color.White,
    background = Color(0xFF0F141B),
    onBackground = Color(0xFFE2E8F0),
    surface = Color(0xFF151B24),
    onSurface = Color(0xFFE2E8F0),
    surfaceVariant = Color(0xFF1D2531),
    onSurfaceVariant = Color(0xFF94A3B8),
    outline = Color(0xFF334155),
    outlineVariant = Color(0xFF1E293B),
    error = Color(0xFFEF4444),
    onError = Color.White,
    inverseSurface = Color(0xFFE2E8F0),
    inverseOnSurface = Color(0xFF0F141B),
    inversePrimary = PurpleDark,
    scrim = Color(0xFF000000),
  )

private val LightColorScheme =
  lightColorScheme(
    primary = Purple,
    onPrimary = Color.White,
    primaryContainer = Color(0xFFF3E8FF),
    onPrimaryContainer = PurpleDark,
    secondary = Cyan,
    onSecondary = Color.White,
    secondaryContainer = Color(0xFFCFFAFE),
    onSecondaryContainer = CyanDark,
    tertiary = PurpleLight,
    onTertiary = Color.White,
    background = Color(0xFFF8FAFC),
    onBackground = Color(0xFF0F172A),
    surface = Color.White,
    onSurface = Color(0xFF0F172A),
    surfaceVariant = Color(0xFFF1F5F9),
    onSurfaceVariant = Color(0xFF64748B),
    outline = Color(0xFFCBD5E1),
    outlineVariant = Color(0xFFE2E8F0),
    error = Color(0xFFEF4444),
    onError = Color.White,
    inverseSurface = Color(0xFF0F172A),
    inverseOnSurface = Color(0xFFF1F5F9),
    inversePrimary = PurpleLight,
    scrim = Color(0xFF000000),
  )

// ============================================================================
// Typography - Modern Sans-Serif (Inter-like)
// ============================================================================

private val CorvusTypography =
  Typography(
    displayLarge =
      TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Bold,
        fontSize = 57.sp,
        lineHeight = 64.sp,
        letterSpacing = (-0.25).sp,
      ),
    displayMedium =
      TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Bold,
        fontSize = 45.sp,
        lineHeight = 52.sp,
        letterSpacing = 0.sp,
      ),
    displaySmall =
      TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.SemiBold,
        fontSize = 36.sp,
        lineHeight = 44.sp,
        letterSpacing = 0.sp,
      ),
    headlineLarge =
      TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.SemiBold,
        fontSize = 32.sp,
        lineHeight = 40.sp,
        letterSpacing = 0.sp,
      ),
    headlineMedium =
      TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.SemiBold,
        fontSize = 28.sp,
        lineHeight = 36.sp,
        letterSpacing = 0.sp,
      ),
    headlineSmall =
      TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.SemiBold,
        fontSize = 24.sp,
        lineHeight = 32.sp,
        letterSpacing = 0.sp,
      ),
    titleLarge =
      TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.SemiBold,
        fontSize = 22.sp,
        lineHeight = 28.sp,
        letterSpacing = 0.sp,
      ),
    titleMedium =
      TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Medium,
        fontSize = 16.sp,
        lineHeight = 24.sp,
        letterSpacing = 0.15.sp,
      ),
    titleSmall =
      TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Medium,
        fontSize = 14.sp,
        lineHeight = 20.sp,
        letterSpacing = 0.1.sp,
      ),
    bodyLarge =
      TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Normal,
        fontSize = 16.sp,
        lineHeight = 24.sp,
        letterSpacing = 0.5.sp,
      ),
    bodyMedium =
      TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Normal,
        fontSize = 14.sp,
        lineHeight = 20.sp,
        letterSpacing = 0.25.sp,
      ),
    bodySmall =
      TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Normal,
        fontSize = 12.sp,
        lineHeight = 16.sp,
        letterSpacing = 0.4.sp,
      ),
    labelLarge =
      TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Medium,
        fontSize = 14.sp,
        lineHeight = 20.sp,
        letterSpacing = 0.1.sp,
      ),
    labelMedium =
      TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Medium,
        fontSize = 12.sp,
        lineHeight = 16.sp,
        letterSpacing = 0.5.sp,
      ),
    labelSmall =
      TextStyle(
        fontFamily = FontFamily.SansSerif,
        fontWeight = FontWeight.Medium,
        fontSize = 11.sp,
        lineHeight = 16.sp,
        letterSpacing = 0.5.sp,
      ),
  )

// ============================================================================
// Custom Shapes (Futuristic - Rounded)
// ============================================================================

private val CorvusShapes =
  Shapes(
    extraSmall = RoundedCornerShape(8.dp),
    small = RoundedCornerShape(12.dp),
    medium = RoundedCornerShape(16.dp),
    large = RoundedCornerShape(20.dp),
    extraLarge = RoundedCornerShape(28.dp),
  )

private data class Shapes(
  val extraSmall: RoundedCornerShape,
  val small: RoundedCornerShape,
  val medium: RoundedCornerShape,
  val large: RoundedCornerShape,
  val extraLarge: RoundedCornerShape,
)

// ============================================================================
// Corvus Design Tokens (for custom components)
// ============================================================================

@Immutable
data class CorvusTokens(
  // Gradient Brushes
  val gradientPrimary: Brush = Brush.horizontalGradient(GradientPurpleCyan),
  val gradientVertical: Brush = Brush.verticalGradient(GradientPurpleCyan),
  val gradientAccent: Brush = Brush.horizontalGradient(GradientCyanPurple),

  // Glow Colors
  val glowPurple: Color = GlowPurple,
  val glowCyan: Color = GlowCyan,

  // Glass Effect
  val glassBackground: Color = SurfaceGlass,
  val glassBorder: Color = Color(0x33FFFFFF),

  // Status Colors
  val statusConnected: Color = SuccessGreen,
  val statusDisconnected: Color = ErrorRed,
  val statusProcessing: Color = WarningAmber,

  // Spacing
  val spacingXs: Int = 4,
  val spacingSm: Int = 8,
  val spacingMd: Int = 16,
  val spacingLg: Int = 24,
  val spacingXl: Int = 32,

  // Border Radius
  val radiusSm: Int = 8,
  val radiusMd: Int = 16,
  val radiusLg: Int = 24,
  val radiusFull: Int = 9999,

  // Animation
  val animationFast: Int = 150,
  val animationNormal: Int = 300,
  val animationSlow: Int = 500,
)

val LocalCorvusTokens = staticCompositionLocalOf { CorvusTokens() }

// ============================================================================
// Theme Provider
// ============================================================================

@Composable
fun CorvusTheme(
  useDarkTheme: Boolean = androidx.compose.foundation.isSystemInDarkTheme(),
  content: @Composable () -> Unit,
) {
  val colorScheme = if (useDarkTheme) DarkColorScheme else LightColorScheme
  val tokens = CorvusTokens()

  CompositionLocalProvider(LocalCorvusTokens provides tokens) {
    MaterialTheme(colorScheme = colorScheme, typography = CorvusTypography, content = content)
  }
}

// ============================================================================
// Extension Properties for easy access
// ============================================================================

object CorvusTheme {
  val tokens: CorvusTokens
    @Composable get() = LocalCorvusTokens.current

  val gradient: CorvusTokens
    @Composable get() = LocalCorvusTokens.current

  val colors: CorvusColors
    @Composable get() = CorvusColors
}
