package com.profiletailors.corvus.ui.theme

import androidx.compose.runtime.Immutable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.graphics.Color

// ============================================================================
// Corvus AI - Futuristic Tech Design System
// Primary Gradient: Purple (#8B5CF6) to Cyan (#06B6D4)
// ============================================================================

// --- Primary Gradient Colors ---
val Purple = Color(0xFF8B5CF6)
val PurpleLight = Color(0xFFA78BFA)
val PurpleDark = Color(0xFF7C3AED)

val Cyan = Color(0xFF06B6D4)
val CyanLight = Color(0xFF22D3EE)
val CyanDark = Color(0xFF0891B2)

// --- Gradient Brush Colors (for Compose) ---
val GradientPurpleCyan = listOf(Purple, Cyan)
val GradientCyanPurple = listOf(Cyan, Purple)
val GradientPurpleLight = listOf(PurpleLight, CyanLight)

// --- Accent Colors ---
val GlowPurple = Color(0xFF8B5CF6)
val GlowCyan = Color(0xFF06B6D4)
val SuccessGreen = Color(0xFF10B981)
val WarningAmber = Color(0xFFF59E0B)
val ErrorRed = Color(0xFFEF4444)

@Immutable
data class CorvusColorPalette(
  val gradientPrimary: List<Color>,
  val gradientAccent: List<Color>,
  val glowPurple: Color,
  val glowCyan: Color,
  val connected: Color,
  val disconnected: Color,
  val processing: Color,
  val glassSurface: Color,
  val glassOverlay: Color,
  val userBubble: Color,
  val userBubbleBackground: Color,
  val aiBubble: Color,
  val aiBubbleBackground: Color,
)

val DarkCorvusColors =
  CorvusColorPalette(
    gradientPrimary = GradientPurpleCyan,
    gradientAccent = GradientCyanPurple,
    glowPurple = GlowPurple,
    glowCyan = GlowCyan,
    connected = SuccessGreen,
    disconnected = ErrorRed,
    processing = WarningAmber,
    glassSurface = Color(0xFF1A1F2E),
    glassOverlay = Color(0x33FFFFFF),
    userBubble = Purple,
    userBubbleBackground = PurpleDark.copy(alpha = 0.3f),
    aiBubble = Cyan,
    aiBubbleBackground = CyanDark.copy(alpha = 0.3f),
  )

val LightCorvusColors =
  CorvusColorPalette(
    gradientPrimary = GradientPurpleLight,
    gradientAccent = GradientCyanPurple,
    glowPurple = PurpleDark,
    glowCyan = CyanDark,
    connected = SuccessGreen,
    disconnected = ErrorRed,
    processing = WarningAmber,
    glassSurface = Color(0xFFF8FAFC),
    glassOverlay = Color(0x1A0F172A),
    userBubble = PurpleLight,
    userBubbleBackground = Purple.copy(alpha = 0.12f),
    aiBubble = CyanDark,
    aiBubbleBackground = Cyan.copy(alpha = 0.12f),
  )

val LocalCorvusColors = staticCompositionLocalOf { DarkCorvusColors }
