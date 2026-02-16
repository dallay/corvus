package com.profiletailors.corvus.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.sp

private val LightColorScheme =
  lightColorScheme(
    primary = Color(0xFF18B28D),
    onPrimary = Color(0xFFFFFFFF),
    primaryContainer = Color(0xFFD8F3EB),
    onPrimaryContainer = Color(0xFF073328),
    secondary = Color(0xFFEAF2FA),
    onSecondary = Color(0xFF1E2A38),
    secondaryContainer = Color(0xFFDCE8F6),
    onSecondaryContainer = Color(0xFF2A3A4A),
    tertiary = Color(0xFF3D82D9),
    onTertiary = Color(0xFFFFFFFF),
    background = Color(0xFFEAF0F7),
    onBackground = Color(0xFF17212D),
    surface = Color(0xFFFCFDFE),
    onSurface = Color(0xFF17212D),
    surfaceVariant = Color(0xFFE1E9F2),
    onSurfaceVariant = Color(0xFF5A6879),
    outline = Color(0xFFC6D4E3),
    outlineVariant = Color(0xFFD6E0EB),
    error = Color(0xFFC73B47),
    onError = Color(0xFFFFFFFF),
  )

private val DarkColorScheme =
  darkColorScheme(
    primary = Color(0xFF1FC39B),
    onPrimary = Color(0xFF032920),
    primaryContainer = Color(0xFF0C4D3E),
    onPrimaryContainer = Color(0xFFBDF1E3),
    secondary = Color(0xFF2A3544),
    onSecondary = Color(0xFFD7DFEA),
    secondaryContainer = Color(0xFF212A37),
    onSecondaryContainer = Color(0xFFB8C6D8),
    tertiary = Color(0xFF5A95E2),
    onTertiary = Color(0xFF071A34),
    background = Color(0xFF0F141B),
    onBackground = Color(0xFFD9E2ED),
    surface = Color(0xFF151B24),
    onSurface = Color(0xFFD9E2ED),
    surfaceVariant = Color(0xFF1D2531),
    onSurfaceVariant = Color(0xFFA1AFBF),
    outline = Color(0xFF334153),
    outlineVariant = Color(0xFF293444),
    error = Color(0xFFFF8992),
    onError = Color(0xFF69000D),
  )

private val BaseTypography = Typography()

private val CorvusTypography =
  BaseTypography.copy(
    displaySmall =
      BaseTypography.displaySmall.copy(
        fontFamily = FontFamily.Serif,
        fontWeight = FontWeight.SemiBold,
      ),
    headlineSmall =
      BaseTypography.headlineSmall.copy(
        fontFamily = FontFamily.Serif,
        fontWeight = FontWeight.SemiBold,
        letterSpacing = (-0.2).sp,
      ),
    titleLarge =
      BaseTypography.titleLarge.copy(
        fontFamily = FontFamily.Serif,
        fontWeight = FontWeight.SemiBold,
      ),
    bodyLarge =
      BaseTypography.bodyLarge.copy(fontFamily = FontFamily.SansSerif, lineHeight = 22.sp),
    labelMedium =
      BaseTypography.labelMedium.copy(fontFamily = FontFamily.Monospace, letterSpacing = 0.15.sp),
  )

@Composable
fun CorvusTheme(useDarkTheme: Boolean = isSystemInDarkTheme(), content: @Composable () -> Unit) {
  MaterialTheme(
    colorScheme = if (useDarkTheme) DarkColorScheme else LightColorScheme,
    typography = CorvusTypography,
    content = content,
  )
}
