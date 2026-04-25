package com.profiletailors.corvus.ui.theme

import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Shapes
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

private const val LIGHT_PRIMARY_CONTAINER_HEX = 0xFFF3E8FF
private const val LIGHT_SECONDARY_CONTAINER_HEX = 0xFFCFFAFE
private const val LIGHT_BACKGROUND_HEX = 0xFFF8FAFC
private const val DARK_BACKGROUND_HEX = 0xFF0F141B
private const val DARK_SURFACE_HEX = 0xFF151B24
private const val DARK_SURFACE_VARIANT_HEX = 0xFF1D2531
private const val DARK_OUTLINE_HEX = 0xFF334155
private const val DARK_OUTLINE_VARIANT_HEX = 0xFF1E293B
private const val DARK_ON_BACKGROUND_HEX = 0xFFE2E8F0
private const val DARK_ON_SURFACE_VARIANT_HEX = 0xFF94A3B8
private const val ERROR_HEX = 0xFFEF4444
private const val LIGHT_ON_BACKGROUND_HEX = 0xFF0F172A
private const val LIGHT_SURFACE_VARIANT_HEX = 0xFFF1F5F9
private const val LIGHT_ON_SURFACE_VARIANT_HEX = 0xFF64748B
private const val LIGHT_OUTLINE_HEX = 0xFFCBD5E1
private const val LIGHT_OUTLINE_VARIANT_HEX = 0xFFE2E8F0
private const val SCRIM_HEX = 0xFF000000
private const val GLASS_BORDER_HEX = 0x33FFFFFF
private const val SPACING_XS = 4
private const val SPACING_SM = 8
private const val SPACING_MD = 16
private const val SPACING_LG = 24
private const val SPACING_XL = 32
private const val RADIUS_SM = 8
private const val RADIUS_MD = 16
private const val RADIUS_LG = 24
private const val RADIUS_FULL = 9999
private const val ANIMATION_FAST_MS = 150
private const val ANIMATION_NORMAL_MS = 300
private const val ANIMATION_SLOW_MS = 500

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
    background = Color(DARK_BACKGROUND_HEX),
    onBackground = Color(DARK_ON_BACKGROUND_HEX),
    surface = Color(DARK_SURFACE_HEX),
    onSurface = Color(DARK_ON_BACKGROUND_HEX),
    surfaceVariant = Color(DARK_SURFACE_VARIANT_HEX),
    onSurfaceVariant = Color(DARK_ON_SURFACE_VARIANT_HEX),
    outline = Color(DARK_OUTLINE_HEX),
    outlineVariant = Color(DARK_OUTLINE_VARIANT_HEX),
    error = Color(ERROR_HEX),
    onError = Color.White,
    inverseSurface = Color(DARK_ON_BACKGROUND_HEX),
    inverseOnSurface = Color(LIGHT_ON_BACKGROUND_HEX),
    inversePrimary = PurpleDark,
    scrim = Color(SCRIM_HEX),
  )

private val LightColorScheme =
  lightColorScheme(
    primary = Purple,
    onPrimary = Color.White,
    primaryContainer = Color(LIGHT_PRIMARY_CONTAINER_HEX),
    onPrimaryContainer = PurpleDark,
    secondary = Cyan,
    onSecondary = Color.White,
    secondaryContainer = Color(LIGHT_SECONDARY_CONTAINER_HEX),
    onSecondaryContainer = CyanDark,
    tertiary = PurpleLight,
    onTertiary = Color.White,
    background = Color(LIGHT_BACKGROUND_HEX),
    onBackground = Color(LIGHT_ON_BACKGROUND_HEX),
    surface = Color.White,
    onSurface = Color(LIGHT_ON_BACKGROUND_HEX),
    surfaceVariant = Color(LIGHT_SURFACE_VARIANT_HEX),
    onSurfaceVariant = Color(LIGHT_ON_SURFACE_VARIANT_HEX),
    outline = Color(LIGHT_OUTLINE_HEX),
    outlineVariant = Color(LIGHT_OUTLINE_VARIANT_HEX),
    error = Color(ERROR_HEX),
    onError = Color.White,
    inverseSurface = Color(LIGHT_ON_BACKGROUND_HEX),
    inverseOnSurface = Color(LIGHT_SURFACE_VARIANT_HEX),
    inversePrimary = PurpleLight,
    scrim = Color(SCRIM_HEX),
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
  val glassBackground: Color = DarkCorvusColors.glassSurface,
  val glassBorder: Color = Color(GLASS_BORDER_HEX),

  // Status Colors
  val statusConnected: Color = SuccessGreen,
  val statusDisconnected: Color = ErrorRed,
  val statusProcessing: Color = WarningAmber,

  // Spacing
  val spacingXs: Int = SPACING_XS,
  val spacingSm: Int = SPACING_SM,
  val spacingMd: Int = SPACING_MD,
  val spacingLg: Int = SPACING_LG,
  val spacingXl: Int = SPACING_XL,

  // Border Radius
  val radiusSm: Int = RADIUS_SM,
  val radiusMd: Int = RADIUS_MD,
  val radiusLg: Int = RADIUS_LG,
  val radiusFull: Int = RADIUS_FULL,

  // Animation
  val animationFast: Int = ANIMATION_FAST_MS,
  val animationNormal: Int = ANIMATION_NORMAL_MS,
  val animationSlow: Int = ANIMATION_SLOW_MS,
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
  val corvusColors = if (useDarkTheme) DarkCorvusColors else LightCorvusColors

  CompositionLocalProvider(
    LocalCorvusTokens provides tokens,
    LocalCorvusColors provides corvusColors,
  ) {
    MaterialTheme(
      colorScheme = colorScheme,
      typography = CorvusTypography,
      shapes = CorvusShapes,
      content = content,
    )
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

  val colors: CorvusColorPalette
    @Composable get() = LocalCorvusColors.current
}
