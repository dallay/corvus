package com.profiletailors.corvus

import kotlin.test.Test
import kotlin.test.assertTrue

class GreetingTest {
  @Test
  fun `should contain platform name in greeting`() {
    val platform = getPlatform()
    val greeting = Greeting().greet()

    assertTrue(greeting.contains("Hello"), "Greeting should start with Hello")
    assertTrue(greeting.contains(platform.name), "Greeting should contain platform name")
  }
}
