package com.profiletailors.corvus.ui.onboarding

import androidx.compose.animation.core.animateFloatAsState
import androidx.compose.animation.core.tween
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
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
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.draw.shadow
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import com.profiletailors.composeapp.generated.resources.Res
import com.profiletailors.composeapp.generated.resources.button_next
import com.profiletailors.composeapp.generated.resources.button_skip
import com.profiletailors.composeapp.generated.resources.button_start
import com.profiletailors.composeapp.generated.resources.onboarding_desc_connect_runtime
import com.profiletailors.composeapp.generated.resources.onboarding_desc_link_surface
import com.profiletailors.composeapp.generated.resources.onboarding_desc_resume_session
import com.profiletailors.composeapp.generated.resources.onboarding_desc_runtime_available
import com.profiletailors.composeapp.generated.resources.onboarding_title_connect_runtime
import com.profiletailors.composeapp.generated.resources.onboarding_title_link_surface
import com.profiletailors.composeapp.generated.resources.onboarding_title_resume_session
import com.profiletailors.composeapp.generated.resources.onboarding_title_runtime_available
import com.profiletailors.corvus.ui.chat.GradientButton
import com.profiletailors.corvus.ui.chat.MobileOnboardingState
import com.profiletailors.corvus.ui.chat.MobileOnboardingStatus
import com.profiletailors.corvus.ui.chat.MobileTransportMode
import com.profiletailors.corvus.ui.chat.MobileTrustMode
import com.profiletailors.corvus.ui.theme.CorvusTheme
import org.jetbrains.compose.resources.StringResource
import org.jetbrains.compose.resources.stringResource

// ============================================================================
// Corvus Onboarding - Futuristic Tech Style
// ============================================================================

internal val OnboardingSkipButtonShape = RoundedCornerShape(8.dp)
internal val OnboardingMainContentShape = RoundedCornerShape(16.dp)
internal val ProgressBarShape = RoundedCornerShape(3.dp)

@Immutable
data class OnboardingStep(
  val titleRes: StringResource,
  val descriptionRes: StringResource,
  val status: MobileOnboardingStatus,
  val trustMode: MobileTrustMode = MobileTrustMode.BRIDGE_LINKED,
  val transportMode: MobileTransportMode = MobileTransportMode.CLI_BRIDGE,
  val icon: OnboardingIcon = OnboardingIcon.SESSION,
  val actionLabel: StringResource = Res.string.button_next,
  val progressIndex: Int = 0,
  val totalSteps: Int = 4,
  val isTerminal: Boolean = false,
)

enum class OnboardingIcon {
  RUNTIME,
  LINK,
  SYNC,
  SESSION,
}

object OnboardingDefaults {
  private const val STEP_INDEX_RUNTIME_AVAILABLE = 0
  private const val STEP_INDEX_LINK_SURFACE = 1
  private const val STEP_INDEX_CONNECT_RUNTIME = 2
  internal const val STEP_INDEX_RESUME_SESSION = 3
  internal const val PROGRESS_BAR_ACTIVE_WIDTH = 32f
  internal const val PROGRESS_BAR_INACTIVE_WIDTH = 16f
  internal const val PROGRESS_BAR_INACTIVE_ALPHA = 0.3f
  internal const val PROGRESS_ANIMATION_DURATION_MS = 300

  val steps: List<OnboardingStep> =
    listOf(
      OnboardingStep(
        titleRes = Res.string.onboarding_title_runtime_available,
        descriptionRes = Res.string.onboarding_desc_runtime_available,
        status = MobileOnboardingStatus.RUNTIME_PATH_CONFIRMED,
        icon = OnboardingIcon.RUNTIME,
        progressIndex = STEP_INDEX_RUNTIME_AVAILABLE,
      ),
      OnboardingStep(
        titleRes = Res.string.onboarding_title_link_surface,
        descriptionRes = Res.string.onboarding_desc_link_surface,
        status = MobileOnboardingStatus.TRUST_PENDING,
        icon = OnboardingIcon.LINK,
        progressIndex = STEP_INDEX_LINK_SURFACE,
      ),
      OnboardingStep(
        titleRes = Res.string.onboarding_title_connect_runtime,
        descriptionRes = Res.string.onboarding_desc_connect_runtime,
        status = MobileOnboardingStatus.TRANSPORT_CONNECTING,
        icon = OnboardingIcon.SYNC,
        progressIndex = STEP_INDEX_CONNECT_RUNTIME,
      ),
      OnboardingStep(
        titleRes = Res.string.onboarding_title_resume_session,
        descriptionRes = Res.string.onboarding_desc_resume_session,
        status = MobileOnboardingStatus.SESSION_PENDING,
        icon = OnboardingIcon.SESSION,
        actionLabel = Res.string.button_start,
        progressIndex = STEP_INDEX_RESUME_SESSION,
        isTerminal = true,
      ),
    )
}

@Immutable data class OnboardingScreenState(val step: OnboardingStep)

@Immutable data class OnboardingScreenActions(val onSkip: () -> Unit, val onNext: () -> Unit)

fun runtimeOnboardingStep(state: MobileOnboardingState): OnboardingStep =
  when (state.status) {
    MobileOnboardingStatus.TARGET_SELECTED -> OnboardingDefaults.steps[0]
    MobileOnboardingStatus.RECOVERY ->
      OnboardingDefaults.steps[2].copy(
        status = MobileOnboardingStatus.RECOVERY,
        icon = OnboardingIcon.SYNC,
        actionLabel = Res.string.button_next,
        progressIndex = 2,
      )
    MobileOnboardingStatus.RUNTIME_PATH_CONFIRMED -> OnboardingDefaults.steps[0]
    MobileOnboardingStatus.TRUST_PENDING ->
      OnboardingDefaults.steps[1].copy(actionLabel = Res.string.button_next)
    MobileOnboardingStatus.TRANSPORT_CONNECTING -> OnboardingDefaults.steps[2]
    MobileOnboardingStatus.SESSION_PENDING ->
      OnboardingDefaults.steps[OnboardingDefaults.STEP_INDEX_RESUME_SESSION]
    MobileOnboardingStatus.SESSION_READY ->
      OnboardingDefaults.steps[OnboardingDefaults.STEP_INDEX_RESUME_SESSION]
    MobileOnboardingStatus.BLOCKED ->
      OnboardingDefaults.steps[2].copy(
        titleRes = Res.string.onboarding_title_connect_runtime,
        descriptionRes = Res.string.onboarding_desc_connect_runtime,
        status = MobileOnboardingStatus.BLOCKED,
        icon = OnboardingIcon.SYNC,
        actionLabel = Res.string.button_next,
        progressIndex = 2,
      )
  }

// ============================================================================
// Onboarding Screen - Main
// ============================================================================

@Composable
fun OnboardingScreen(
  state: OnboardingScreenState,
  actions: OnboardingScreenActions,
  modifier: Modifier = Modifier,
) {
  val layoutModifier = onboardingLayoutModifier()

  Column(
    modifier = modifier.then(layoutModifier),
    horizontalAlignment = Alignment.CenterHorizontally,
    verticalArrangement = Arrangement.SpaceBetween,
  ) {
    OnboardingSkipButton(onSkip = actions.onSkip)
    OnboardingMainContent(step = state.step)
    OnboardingFooter(
      currentStep = state.step.progressIndex,
      totalSteps = state.step.totalSteps,
      actionLabel = stringResource(state.step.actionLabel),
      onNext = actions.onNext,
    )
  }
}

@Composable
private fun onboardingLayoutModifier(): Modifier {
  val colors = MaterialTheme.colorScheme
  val corvusColors = CorvusTheme.colors

  val backgroundBrush =
    remember(colors.background, corvusColors.glassSurface) {
      Brush.verticalGradient(
        colors = listOf(colors.background, corvusColors.glassSurface, colors.background)
      )
    }

  return remember(backgroundBrush) {
    Modifier.fillMaxSize()
      .background(brush = backgroundBrush)
      .safeContentPadding()
      .padding(horizontal = 24.dp, vertical = 32.dp)
  }
}

@Suppress("FunctionNaming") // Composable functions follow PascalCase per Compose conventions
@Composable
private fun OnboardingSkipButton(onSkip: () -> Unit) {
  val colors = MaterialTheme.colorScheme

  Box(modifier = Modifier.fillMaxWidth(), contentAlignment = Alignment.CenterEnd) {
    Text(
      text = stringResource(Res.string.button_skip),
      style = MaterialTheme.typography.labelLarge,
      color = colors.onSurfaceVariant.copy(alpha = 0.7f),
      modifier =
        Modifier.clip(OnboardingSkipButtonShape)
          .clickable(role = Role.Button, onClick = onSkip)
          .background(Color.Transparent)
          .padding(8.dp),
    )
  }
}

@Suppress("FunctionNaming") // Composable functions follow PascalCase per Compose conventions
@Composable
private fun ColumnScope.OnboardingMainContent(step: OnboardingStep) {
  val colors = MaterialTheme.colorScheme
  val corvusColors = CorvusTheme.colors

  Column(
    horizontalAlignment = Alignment.CenterHorizontally,
    verticalArrangement = Arrangement.Center,
    modifier = Modifier.weight(1f),
  ) {
    Spacer(modifier = Modifier.height(48.dp))
    OnboardingIconDisplay(icon = step.icon)
    Spacer(modifier = Modifier.height(48.dp))

    Box(
      modifier =
        Modifier.shadow(
          elevation = 8.dp,
          shape = OnboardingMainContentShape,
          spotColor = corvusColors.glowPurple.copy(alpha = 0.3f),
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
    Text(
      text = stringResource(step.descriptionRes),
      style = MaterialTheme.typography.bodyLarge,
      color = colors.onSurfaceVariant,
      textAlign = TextAlign.Center,
      modifier = Modifier.padding(horizontal = 16.dp),
    )
  }
}

@Suppress("FunctionNaming") // Composable functions follow PascalCase per Compose conventions
@Composable
private fun OnboardingFooter(
  currentStep: Int,
  totalSteps: Int,
  actionLabel: String,
  onNext: () -> Unit,
) {
  Column(modifier = Modifier.fillMaxWidth(), horizontalAlignment = Alignment.CenterHorizontally) {
    FuturisticProgressIndicator(currentStep = currentStep, totalSteps = totalSteps)
    Spacer(modifier = Modifier.height(32.dp))
    GradientButton(
      text = actionLabel,
      onClick = onNext,
      modifier = Modifier.fillMaxWidth().height(56.dp),
    )
  }
}

// ============================================================================
// Onboarding Icon - Animated with Glow
// ============================================================================

@Composable
private fun OnboardingIconDisplay(icon: OnboardingIcon) {
  val corvusColors = CorvusTheme.colors
  val gradient =
    remember(corvusColors.gradientPrimary) { Brush.linearGradient(corvusColors.gradientPrimary) }

  Box(
    modifier =
      Modifier.size(120.dp)
        .shadow(
          elevation = 24.dp,
          shape = CircleShape,
          spotColor = corvusColors.glowPurple.copy(alpha = 0.4f),
        )
        .background(brush = gradient, shape = CircleShape),
    contentAlignment = Alignment.Center,
  ) {
    // Inner glow ring
    val innerGlowBrush = remember {
      Brush.radialGradient(colors = listOf(Color.White.copy(alpha = 0.3f), Color.Transparent))
    }
    Box(modifier = Modifier.size(100.dp).background(brush = innerGlowBrush, shape = CircleShape))

    // Icon or Letter
    Box(contentAlignment = Alignment.Center) {
      Text(
        text =
          when (icon) {
            OnboardingIcon.RUNTIME -> "⚙"
            OnboardingIcon.LINK -> "🔗"
            OnboardingIcon.SYNC -> "📡"
            OnboardingIcon.SESSION -> "💬"
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
  val corvusColors = CorvusTheme.colors
  val activeBrush =
    remember(corvusColors.gradientPrimary) {
      Brush.horizontalGradient(corvusColors.gradientPrimary)
    }
  val outlineColor = MaterialTheme.colorScheme.outline

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
              isActive -> OnboardingDefaults.PROGRESS_BAR_ACTIVE_WIDTH
              isCompleted -> OnboardingDefaults.PROGRESS_BAR_INACTIVE_WIDTH
              else -> OnboardingDefaults.PROGRESS_BAR_INACTIVE_WIDTH
            },
          animationSpec = tween(OnboardingDefaults.PROGRESS_ANIMATION_DURATION_MS),
          label = "progressWidth",
        )

      val alpha by
        animateFloatAsState(
          targetValue =
            if (isActive || isCompleted) 1f else OnboardingDefaults.PROGRESS_BAR_INACTIVE_ALPHA,
          animationSpec = tween(OnboardingDefaults.PROGRESS_ANIMATION_DURATION_MS),
          label = "progressAlpha",
        )

      Box(
        modifier =
          Modifier.width(width.dp)
            .height(6.dp)
            .shadow(
              elevation = if (isActive) 4.dp else 0.dp,
              shape = ProgressBarShape,
              spotColor = corvusColors.glowPurple.copy(alpha = alpha * 0.5f),
            )
            .clip(ProgressBarShape)
            .then(
              if (isActive || isCompleted) {
                Modifier.background(brush = activeBrush)
              } else {
                Modifier.background(color = outlineColor.copy(alpha = alpha))
              }
            )
      )
    }
  }
}
