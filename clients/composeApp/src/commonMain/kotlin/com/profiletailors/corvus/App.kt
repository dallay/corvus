package com.profiletailors.corvus

import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableIntStateOf
import androidx.compose.runtime.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.tooling.preview.Preview
import com.profiletailors.corvus.ui.chat.ChatWorkspace
import com.profiletailors.corvus.ui.chat.ChatWorkspaceDefaults
import com.profiletailors.corvus.ui.onboarding.OnboardingDefaults
import com.profiletailors.corvus.ui.onboarding.OnboardingScreen
import com.profiletailors.corvus.ui.theme.CorvusTheme

private const val AgentName = "Corvus Agent"

@Composable
@Preview
fun App() {
  val platform = getPlatform()
  var onboardingStepIndex by rememberSaveable { mutableIntStateOf(0) }
  val onboardingSteps = OnboardingDefaults.steps()
  val shouldShowOnboarding = platform.isMobile && onboardingStepIndex < onboardingSteps.size

  CorvusTheme {
    if (shouldShowOnboarding) {
      OnboardingScreen(
        step = onboardingSteps[onboardingStepIndex],
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
      ChatWorkspace(state = ChatWorkspaceDefaults.state(modelName = AgentName))
    }
  }
}
