package com.profiletailors.plugin.utils

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test

class TextHandleTest {

  /**
   * ```
   * Test empty string input
   * Expected: Returns empty string
   * ```
   */
  @Test
  fun `wrapText with empty string should return empty string`() {
    val result = TextHandle.wrapText("")
    assertEquals("", result)
  }

  /**
   * ```
   * Test single word input
   * Expected: Returns the original word
   * ```
   */
  @Test
  fun `wrapText with single word should return the word`() {
    val result = TextHandle.wrapText("Hello", 10)
    assertEquals("Hello", result)
  }

  /**
   * ```
   * Test multiple short words within line width
   * Expected: All words on same line, separated by spaces
   * ```
   */
  @Test
  fun `wrapText with multiple short words within line width should keep them in one line`() {
    val result = TextHandle.wrapText("Hello world test", 20)
    assertEquals("Hello world test", result)
  }

  /**
   * ```
   * Test word longer than line width
   * Expected: Long word wraps to new line
   * ```
   */
  @Test
  fun `wrapText with word longer than line width should wrap the word`() {
    val result = TextHandle.wrapText("Hello verylongword test", 10)
    val expected = "Hello\nverylongword\ntest"
    assertEquals(expected, result)
  }

  /**
   * ```
   * Test text exactly matching line width
   * Expected: Fills line exactly, no wrapping needed
   * ```
   */
  @Test
  fun `wrapText with text exactly matching line width should not wrap`() {
    val result = TextHandle.wrapText("Hello world", 11) // "Hello world" is 11 characters
    assertEquals("Hello world", result)
  }

  /**
   * ```
   * Test line width of 1
   * Expected: Each word wraps to new line
   * ```
   */
  @Test
  fun `wrapText with line width of 1 should wrap each word`() {
    val result = TextHandle.wrapText("A B C", 1)
    val expected = "A\nB\nC"
    assertEquals(expected, result)
  }

  /**
   * ```
   * Test default line width (80)
   * Expected: Uses default line width for wrapping
   * ```
   */
  @Test
  fun `wrapText with default line width should use 80 characters`() {
    val longText = "A".repeat(79) + " B" // 79 As + space + B = 81 characters
    val result = TextHandle.wrapText(longText)
    val expected = "${"A".repeat(79)}\nB"
    assertEquals(expected, result)
  }

  /**
   * ```
   * Test multiple consecutive spaces
   * Expected: Multiple spaces normalized to single space
   * ```
   */
  @Test
  fun `wrapText should handle multiple spaces by normalizing them`() {
    val result = TextHandle.wrapText("Hello    world    test", 20)
    assertEquals("Hello world test", result)
  }

  /**
   * ```
   * Test text containing newlines
   * Expected: Original newlines preserved and normalized
   * ```
   */
  @Test
  fun `wrapText should handle text with newlines`() {
    val result = TextHandle.wrapText("Hello\n\nworld", 10)
    assertEquals("Hello\nworld", result)
  }

  /**
   * ```
   * Test very long word requiring forced wrap
   * Expected: Long word on its own line
   * ```
   */
  @Test
  fun `wrapText with very long word should place it on separate line`() {
    val result = TextHandle.wrapText("short supercalifragilisticexpialidocious end", 15)
    val expected = "short\nsupercalifragilisticexpialidocious\nend"
    assertEquals(expected, result)
  }

  /**
   * ```
   * Test last line handling
   * Expected: Last line correctly added to result
   * ```
   */
  @Test
  fun `wrapText should properly handle the last line`() {
    val result = TextHandle.wrapText("one two three", 7)
    val expected = "one two\nthree"
    assertEquals(expected, result)
  }

  /**
   * ```
   * Test string with only spaces
   * Expected: Returns empty string
   * ```
   */
  @Test
  fun `wrapText with only spaces should return empty string`() {
    val result = TextHandle.wrapText("   ")
    assertEquals("", result)
  }

  /**
   * ```
   * Test line width exactly fitting word plus space
   * Expected: Wraps correctly
   * ```
   */
  @Test
  fun `wrapText with exact fit including space should wrap correctly`() {
    // "Hello"(5) + " "(1) + "world"(5) = 11 characters
    val result = TextHandle.wrapText("Hello world test", 11)
    val expected = "Hello world\ntest"
    assertEquals(expected, result)
  }
}
