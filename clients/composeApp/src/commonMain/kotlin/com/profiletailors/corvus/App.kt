package com.profiletailors.corvus

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.tooling.preview.Preview
import com.profiletailors.corvus.ui.chat.ChatWorkspace
import com.profiletailors.corvus.ui.chat.ChatWorkspaceDefaults
import com.profiletailors.corvus.ui.onboarding.OnboardingDefaults
import com.profiletailors.corvus.ui.onboarding.OnboardingScreen
import com.profiletailors.corvus.ui.theme.CorvusTheme

private const val AGENT_NAME = "Corvus Agent"

@Composable
@Preview
fun App() {
  val platform = remember { getPlatform() }
  var onboardingStepIndex by rememberSaveable { mutableIntStateOf(0) }
  val onboardingSteps = remember { OnboardingDefaults.steps }
  val shouldShowOnboarding =
    remember(platform, onboardingStepIndex, onboardingSteps) {
      platform.isMobile && onboardingStepIndex < onboardingSteps.size
    }

  CorvusTheme {
    if (shouldShowOnboarding) {
      OnboardingScreen(
        step = onboardingSteps[onboardingStepIndex],
        currentStepIndex = onboardingStepIndex,
        totalSteps = onboardingSteps.size,
        isLastStep = onboardingStepIndex == onboardingSteps.lastIndex,
        onSkip = { onboardingStepIndex = onboardingSteps.size },
        onNext = {
          onboardingStepIndex =
            if (onboardingStepIndex < onboardingSteps.lastIndex) {
              onboardingStepIndex + 1
            } else {
              onboardingSteps.size
            }
        },
      )
    } else {
      ChatWorkspace(state = ChatWorkspaceDefaults.state(modelName = AGENT_NAME))
    }
  }
}
