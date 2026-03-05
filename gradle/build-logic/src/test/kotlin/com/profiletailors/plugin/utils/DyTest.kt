package com.profiletailors.plugin.utils

import java.time.Duration
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class ChunkedVirtualTest {

  @Test
  fun successCaseShouldReturnDoubledValues() {
    val input = listOf(1, 2, 3, 4, 5)
    val result = input.chunkedVirtual(size = 2) { it * 2 }
    assertEquals(listOf(2, 4, 6, 8, 10), result)
  }

  @Test
  fun failureCaseShouldThrowExceptionOnFail() {
    val input = listOf(1, 2, 3)
    assertThrows(IllegalStateException::class.java) {
      input.chunkedVirtual(size = 2) {
        if (it == 2) error("fail on 2")
        it * 2
      }
    }
  }

  @Test
  fun timeoutShouldCancelTask() {
    val input = listOf(1, 2, 3)
    val exception =
      assertThrows(IllegalStateException::class.java) {
        input.chunkedVirtual(size = 2, timeout = Duration.ofMillis(10)) {
          Thread.sleep(100)
          it * 2
        }
      }
    assertTrue(exception.message?.contains("cancelled") ?: false)
  }

  @Test
  fun preservesOrderOfResults() {
    val input = (1..10).toList()
    val result = input.chunkedVirtual(size = 3) { it * it }
    assertEquals(input.map { it * it }, result)
  }

  @Test
  fun emptyInputReturnsEmpty() {
    val input = emptyList<Int>()
    val result = input.chunkedVirtual { it * 2 }
    assertTrue(result.isEmpty())
  }

  @Test
  fun largeScaleStressTest() {
    val input = (1..1000).toList()

    val result =
      input.chunkedVirtual(size = 50, timeout = Duration.ofSeconds(10)) { x ->
        Thread.sleep(1)
        x * 2
      }

    assertEquals(1000, result.size)
    assertEquals((1..1000).map { it * 2 }, result)
  }
}
