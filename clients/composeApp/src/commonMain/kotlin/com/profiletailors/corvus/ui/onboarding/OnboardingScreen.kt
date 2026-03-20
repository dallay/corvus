package com.profiletailors.corvus.ui.onboarding

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeContentPadding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.getValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.profiletailors.composeapp.generated.resources.Res
import com.profiletailors.composeapp.generated.resources.button_next
import com.profiletailors.composeapp.generated.resources.button_skip
import com.profiletailors.composeapp.generated.resources.button_start
import com.profiletailors.composeapp.generated.resources.onboarding_desc_connect_gateway
import com.profiletailors.composeapp.generated.resources.onboarding_desc_talk_agent
import com.profiletailors.composeapp.generated.resources.onboarding_desc_welcome
import com.profiletailors.composeapp.generated.resources.onboarding_title_connect_gateway
import com.profiletailors.composeapp.generated.resources.onboarding_title_talk_agent
import com.profiletailors.composeapp.generated.resources.onboarding_title_welcome
import com.profiletailors.corvus.ui.chat.GradientButton
import com.profiletailors.corvus.ui.theme.CorvusColors
import com.profiletailors.corvus.ui.theme.GradientPurpleCyan
import org.jetbrains.compose.resources.StringResource
import org.jetbrains.compose.resources.stringResource

// ============================================================================
// Corvus Onboarding - Futuristic Tech Style
// ============================================================================

@Immutable
data class OnboardingStep(
  val titleRes: StringResource,
  val descriptionRes: StringResource,
  val icon: OnboardingIcon = OnboardingIcon.NEURAL,
)

enum class OnboardingIcon {
  WAVE, // Welcome
  LINK, // Connect Gateway
  NEURAL, // Talk to Agent
}

object OnboardingDefaults {
  val steps: List<OnboardingStep> =
    listOf(
      OnboardingStep(
        titleRes = Res.string.onboarding_title_welcome,
        descriptionRes = Res.string.onboarding_desc_welcome,
        icon = OnboardingIcon.WAVE,
      ),
      OnboardingStep(
        titleRes = Res.string.onboarding_title_connect_gateway,
        descriptionRes = Res.string.onboarding_desc_connect_gateway,
        icon = OnboardingIcon.LINK,
      ),
      OnboardingStep(
        titleRes = Res.string.onboarding_title_talk_agent,
        descriptionRes = Res.string.onboarding_desc_talk_agent,
        icon = OnboardingIcon.NEURAL,
      ),
    )
}

// ============================================================================
// Onboarding Screen - Main
// ============================================================================

@Composable
fun OnboardingScreen(
  step: OnboardingStep,
  currentStepIndex: Int,
  totalSteps: Int,
  isLastStep: Boolean,
  onSkip: () -> Unit,
  onNext: () -> Unit,
  modifier: Modifier = Modifier,
) {
  val colors = MaterialTheme.colorScheme

  Column(
    modifier =
      modifier
        .fillMaxSize()
        .background(
          brush =
            Brush.verticalGradient(
              colors = listOf(colors.background, CorvusColors.glassSurface, colors.background)
            )
        )
        .safeContentPadding()
        .padding(horizontal = 24.dp, vertical = 32.dp),
    horizontalAlignment = Alignment.CenterHorizontally,
    verticalArrangement = Arrangement.SpaceBetween,
  ) {
    // Skip Button (Top Right)
    Box(modifier = Modifier.fillMaxWidth(), contentAlignment = Alignment.CenterEnd) {
      Text(
        text = stringResource(Res.string.button_skip),
        style = MaterialTheme.typography.labelLarge,
        color = colors.onSurfaceVariant.copy(alpha = 0.7f),
        modifier =
          Modifier.clip(RoundedCornerShape(8.dp)).background(Color.Transparent).padding(8.dp),
      )
    }

    // Main Content
    Column(
      horizontalAlignment = Alignment.CenterHorizontally,
      verticalArrangement = Arrangement.Center,
      modifier = Modifier.weight(1f),
    ) {
      Spacer(modifier = Modifier.height(48.dp))

      // Animated Icon with Glow
      OnboardingIconDisplay(icon = step.icon)

      Spacer(modifier = Modifier.height(48.dp))

      // Gradient Title
      Box(
        modifier =
          Modifier.shadow(
            elevation = 8.dp,
            shape = RoundedCornerShape(16.dp),
            spotColor = CorvusColors.glowPurple.copy(alpha = 0.3f),
          )
      ) {
        Text(
          text = stringResource(step.titleRes),
          style = MaterialTheme.typography.displaySmall,
          fontWeight = FontWeight.Bold,
          color = colors.onBackground,
          textAlign = TextAlign.Center,
          fontFamily = FontFamily.SansSerif,
        )
      }

      Spacer(modifier = Modifier.height(20.dp))

      // Description
      Text(
        text = stringResource(step.descriptionRes),
        style = MaterialTheme.typography.bodyLarge,
        color = colors.onSurfaceVariant,
        textAlign = TextAlign.Center,
        modifier = Modifier.padding(horizontal = 16.dp),
      )
    }

    // Bottom Section
    Column(modifier = Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {
      // Progress Indicator
      FuturisticProgressIndicator(currentStep = currentStepIndex, totalSteps = totalSteps)

      Spacer(modifier = Modifier.height(32.dp))

      // Next/Start Button
      GradientButton(
        text = stringResource(if (isLastStep) Res.string.button_start else Res.string.button_next),
        onClick = onNext,
        modifier = Modifier.fillMaxWidth().height(56.dp),
      )
    }
  }
}

// ============================================================================
// Onboarding Icon - Animated with Glow
// ============================================================================

@Composable
private fun OnboardingIconDisplay(icon: OnboardingIcon) {
  val gradient = Brush.linearGradient(GradientPurpleCyan)

  Box(
    modifier =
      Modifier.size(120.dp)
        .shadow(
          elevation = 24.dp,
          shape = CircleShape,
          spotColor = CorvusColors.glowPurple.copy(alpha = 0.4f),
        )
        .background(brush = gradient, shape = CircleShape),
    contentAlignment = Alignment.Center,
  ) {
    // Inner glow ring
    Box(
      modifier =
        Modifier.size(100.dp)
          .background(
            brush =
              Brush.radialGradient(
                colors = listOf(Color.White.copy(alpha = 0.3f), Color.Transparent)
              ),
            shape = CircleShape,
          )
    )

    // Icon or Letter
    Box(contentAlignment = Alignment.Center) {
      Text(
        text =
          when (icon) {
            OnboardingIcon.WAVE -> "👋"
            OnboardingIcon.LINK -> "🔗"
            OnboardingIcon.NEURAL -> "🧠"
          },
        fontSize = 48.sp,
      )
    }
  }
}

// ============================================================================
// Futuristic Progress Indicator
// ============================================================================

@Composable
private fun FuturisticProgressIndicator(currentStep: Int, totalSteps: Int) {
  Row(
    horizontalArrangement = Arrangement.spacedBy(12.dp),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    repeat(totalSteps) { stepIndex ->
      val isActive = stepIndex == currentStep
      val isCompleted = stepIndex < currentStep

      val width by
        animateFloatAsState(
          targetValue =
            when {
              isActive -> 32f
              isCompleted -> 16f
              else -> 16f
            },
          animationSpec = tween(300),
          label = "progressWidth",
        )

      val alpha by
        animateFloatAsState(
          targetValue = if (isActive || isCompleted) 1f else 0.3f,
          animationSpec = tween(300),
          label = "progressAlpha",
        )

      Box(
        modifier =
          Modifier.width(width.dp)
            .height(6.dp)
            .shadow(
              elevation = if (isActive) 4.dp else 0.dp,
              shape = RoundedCornerShape(3.dp),
              spotColor = CorvusColors.glowPurple.copy(alpha = alpha * 0.5f),
            )
            .clip(RoundedCornerShape(3.dp))
            .background(
              brush =
                if (isActive || isCompleted) {
                  Brush.horizontalGradient(GradientPurpleCyan)
                } else {
                  Brush.horizontalGradient(
                    listOf(
                      MaterialTheme.colorScheme.outline.copy(alpha = alpha),
                      MaterialTheme.colorScheme.outline.copy(alpha = alpha),
                    )
                  )
                }
            )
      )
    }
  }
}

// ============================================================================
// Step Indicator (Legacy - kept for compatibility)
// ============================================================================

@Composable
private fun StepIndicator(currentStepIndex: Int, totalSteps: Int) {
  FuturisticProgressIndicator(currentStep = currentStepIndex, totalSteps = totalSteps)
}
