package com.profiletailors.corvus

import com.profiletailors.corvus.ui.onboarding.OnboardingDefaults
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class OnboardingDefaultsTest {

  @Test
  fun `should expose mobile onboarding steps`() {
    val steps = OnboardingDefaults.steps()

    assertEquals(3, steps.size)
    assertTrue(steps.all { it.title.isNotBlank() && it.description.isNotBlank() })
  }
}
