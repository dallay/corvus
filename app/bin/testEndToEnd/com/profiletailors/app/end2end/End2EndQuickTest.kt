package com.profiletailors.app.end2end

import com.profiletailors.app.mock.api.MockServer
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test

/** Tests the end-to-end quick test. */
class End2EndQuickTest {

  /** Tests something that is quick. */
  @Test
  fun testSome() {
    assertEquals(0, sendStuff(MockServer().get()))
  }

  /**
   * Sends stuff.
   *
   * @param things The things to send.
   * @return The number of things sent.
   */
  private fun sendStuff(things: List<String>): Int {
    return things.size
  }
}
