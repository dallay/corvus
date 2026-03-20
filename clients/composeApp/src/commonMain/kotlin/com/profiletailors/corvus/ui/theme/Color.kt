package com.profiletailors.corvus.ui.theme

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

// --- Surface Colors (Glassmorphism) ---
val SurfaceGlass = Color(0xFF1A1F2E)
val SurfaceGlassLight = Color(0xFF252B3D)
val SurfaceGlassOverlay = Color(0x33FFFFFF)

// --- Accent Colors ---
val GlowPurple = Color(0xFF8B5CF6)
val GlowCyan = Color(0xFF06B6D4)
val SuccessGreen = Color(0xFF10B981)
val WarningAmber = Color(0xFFF59E0B)
val ErrorRed = Color(0xFFEF4444)

// ============================================================================
// Semantic Color Names (for specific use cases)
// ============================================================================

object CorvusColors {
  // Gradient Brush Colors
  val gradientPrimary = GradientPurpleCyan
  val gradientAccent = GradientCyanPurple

  // Glow Effects
  val glowPurple = GlowPurple
  val glowCyan = GlowCyan

  // Status Colors
  val connected = SuccessGreen
  val disconnected = ErrorRed
  val processing = WarningAmber

  // Glass Effect
  val glassSurface = SurfaceGlass
  val glassOverlay = SurfaceGlassOverlay

  // Chat Bubbles
  val userBubble = Purple
  val userBubbleBackground = PurpleDark.copy(alpha = 0.3f)
  val aiBubble = Cyan
  val aiBubbleBackground = CyanDark.copy(alpha = 0.3f)
}
