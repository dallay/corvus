package com.profiletailors.corvus

import com.profiletailors.corvus.ui.onboarding.OnboardingDefaults
import kotlin.test.Test
import kotlin.test.assertEquals

class OnboardingDefaultsTest {

  @Test
  fun `should expose stable mobile onboarding steps`() {
    val steps = OnboardingDefaults.steps

    assertEquals(3, steps.size)
    assertEquals(3, steps.map { it.titleRes }.distinct().size)
    assertEquals(3, steps.map { it.descriptionRes }.distinct().size)
  }
}
