package com.profiletailors.corvus.ui.onboarding

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
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
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
import org.jetbrains.compose.resources.StringResource
import org.jetbrains.compose.resources.stringResource

data class OnboardingStep(
  val titleRes: StringResource,
  val descriptionRes: StringResource,
)

object OnboardingDefaults {
  val steps: List<OnboardingStep> =
    listOf(
      OnboardingStep(
        titleRes = Res.string.onboarding_title_welcome,
        descriptionRes = Res.string.onboarding_desc_welcome,
      ),
      OnboardingStep(
        titleRes = Res.string.onboarding_title_connect_gateway,
        descriptionRes = Res.string.onboarding_desc_connect_gateway,
      ),
      OnboardingStep(
        titleRes = Res.string.onboarding_title_talk_agent,
        descriptionRes = Res.string.onboarding_desc_talk_agent,
      ),
    )
}

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
        .background(colors.background)
        .safeContentPadding()
        .padding(horizontal = 20.dp, vertical = 24.dp),
    horizontalAlignment = Alignment.CenterHorizontally,
    verticalArrangement = Arrangement.SpaceBetween,
  ) {
    Spacer(modifier = Modifier.height(24.dp))

    Column(horizontalAlignment = Alignment.CenterHorizontally) {
      Text(
        text = stringResource(step.titleRes),
        style = MaterialTheme.typography.headlineMedium,
        fontWeight = FontWeight.Bold,
        color = colors.onBackground,
        textAlign = TextAlign.Center,
      )

      Spacer(modifier = Modifier.height(16.dp))

      Text(
        text = stringResource(step.descriptionRes),
        style = MaterialTheme.typography.bodyLarge,
        color = colors.onSurfaceVariant,
        textAlign = TextAlign.Center,
      )
    }

    Column(
      modifier = Modifier.fillMaxWidth(),
      horizontalAlignment = Alignment.CenterHorizontally,
    ) {
      StepIndicator(
        currentStepIndex = currentStepIndex,
        totalSteps = totalSteps,
      )

      Spacer(modifier = Modifier.height(18.dp))

      Row(
        modifier = Modifier.fillMaxWidth(),
        horizontalArrangement = if (isLastStep) Arrangement.End else Arrangement.SpaceBetween,
        verticalAlignment = Alignment.CenterVertically,
      ) {
        if (!isLastStep) {
          TextButton(onClick = onSkip) { Text(text = stringResource(Res.string.button_skip)) }
        }

        Button(onClick = onNext) {
          Text(
            text =
              stringResource(
                if (isLastStep) {
                  Res.string.button_start
                } else {
                  Res.string.button_next
                }
              )
          )
        }
      }
    }
  }
}

@Composable
private fun StepIndicator(currentStepIndex: Int, totalSteps: Int) {
  val colors = MaterialTheme.colorScheme

  Row(
    horizontalArrangement = Arrangement.spacedBy(8.dp),
    verticalAlignment = Alignment.CenterVertically,
  ) {
    repeat(totalSteps) { stepIndex ->
      val isActive = stepIndex == currentStepIndex
      Box(
        modifier =
          Modifier
            .size(if (isActive) 10.dp else 8.dp)
            .clip(CircleShape)
            .background(
              if (isActive) {
                colors.primary
              } else {
                colors.outlineVariant
              }
            ),
      )
    }
  }
}
