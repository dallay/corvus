package com.profiletailors.plugin.utils

import com.profiletailors.fixtures.consoleLog
import java.net.URI
import java.time.Duration
import org.junit.jupiter.api.Test

/** tests for [HttpUtils] */
class HttpUtilsTest {

  @Test
  fun testGoogle() {
    val google =
      runCatching { HttpUtils.get(URI("https://www.google.com"), Duration.ofSeconds(10)) }
        .getOrDefault("Fetch failed") ?: ""

    consoleLog(google)
    assert(google.contains("Google"))
  }
}
