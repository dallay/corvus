package com.profiletailors.plugin.utils

import com.profiletailors.fixtures.consoleLog
import java.net.URI
import java.time.Duration
import org.junit.jupiter.api.Test

/** tests for [HttpUtils] */
class HttpUtilsTest {

  @Test
  fun testBaidu() {
    val baidu =
      runCatching { HttpUtils.get(URI("https://www.baidu.com"), Duration.ofSeconds(10)) }
        .getOrDefault("获取失败") ?: ""

    consoleLog(baidu)
    assert(baidu.contains("百度一下"))
  }
}
