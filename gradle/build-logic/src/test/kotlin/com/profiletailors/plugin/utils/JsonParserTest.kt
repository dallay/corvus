package com.profiletailors.plugin.utils

import com.profiletailors.fixtures.consoleLog
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.MethodOrderer
import org.junit.jupiter.api.Nested
import org.junit.jupiter.api.Order
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.TestMethodOrder
import org.junit.jupiter.api.assertAll
import org.junit.jupiter.api.assertThrows

class JsonParserTest {

  @Nested
  @TestMethodOrder(MethodOrderer.OrderAnnotation::class)
  inner class SuccessTests {

    @Test
    @Order(0)
    fun `success parser tests`() {
      consoleLog("===== Success Parser Tests =====")
    }

    @Test
    @Order(1)
    fun `parse empty object`() {
      val result = JsonParser.parseMap("{}")
      assertEquals(emptyMap<String, Any?>(), result)
      consoleLog("✅ PASS: Empty object")
    }

    @Test
    @Order(2)
    fun `parse simple object`() {
      val json = """{"name":"Alice","age":25,"active":true}"""
      val expected = mapOf("name" to "Alice", "age" to 25, "active" to true)
      val result = JsonParser.parseMap(json)
      assertEquals(expected, result)
      consoleLog("✅ PASS: Simple object")
    }

    @Test
    @Order(3)
    fun `parse nested structures`() {
      val json = """{"user":{"id":1,"prefs":{"theme":"dark"}},"tags":["kotlin","json"]}"""
      val expected =
        mapOf(
          "user" to mapOf("id" to 1, "prefs" to mapOf("theme" to "dark")),
          "tags" to listOf("kotlin", "json"),
        )
      val result = JsonParser.parseMap(json)
      assertEquals(expected, result)
      consoleLog("✅ PASS: Nested structures")
    }

    @Test
    @Order(4)
    fun `parse empty array`() {
      val result = JsonParser.parseList("[]")
      assertEquals(emptyList<Any?>(), result)
      consoleLog("✅ PASS: Empty array")
    }

    @Test
    @Order(5)
    fun `parse mixed type array`() {
      val json = """[null, 42, -3.14, "text", true, false]"""
      val expected = listOf(null, 42, -3.14, "text", true, false)
      val result = JsonParser.parseList(json)
      assertEquals(expected, result)
      consoleLog("✅ PASS: Mixed type array")
    }

    @Test
    @Order(6)
    fun `parse unicode escape`() {
      val json = """"Hello\u0020World!\uD83D\uDE00""""
      val expected = "Hello World!😀"
      val result = JsonParser.parse(json)
      result?.let { assertEquals(expected, it) }
      consoleLog("✅ PASS: Unicode escape")
    }

    @Test
    @Order(7)
    fun `parse scientific notation`() {
      val json = "1.23e4"
      val expected = 12300.0
      val result = JsonParser.parse(json) as Double
      assertEquals(expected, result, 0.001)
      consoleLog("✅ PASS: Scientific notation")
    }
  }

  @Nested
  @TestMethodOrder(MethodOrderer.OrderAnnotation::class)
  inner class ErrorHandlingTests {

    @Test
    @Order(0)
    fun `error handling tests`() {
      consoleLog("===== Error Handling Tests =====")
    }

    @Test
    @Order(1)
    fun `throw on invalid token`() {
      val json = """{"key": undefined}"""
      val exception = assertThrows<JsonParser.JsonException> { JsonParser.parse(json) }
      assertTrue(exception.message!!.contains("line"))
      consoleLog("✅ PASS: Invalid token - ${exception.message}")
    }

    @Test
    @Order(2)
    fun `throw on trailing comma in array`() {
      val json = "[1,,2]"
      val exception = assertThrows<JsonParser.JsonException> { JsonParser.parse(json) }
      assertTrue(exception.message!!.contains("line"))
      consoleLog("✅ PASS: Trailing comma - ${exception.message}")
    }

    @Test
    @Order(3)
    fun `throw on missing comma in object`() {
      val json = """{"a":1 "b":2}"""
      val exception = assertThrows<JsonParser.JsonException> { JsonParser.parse(json) }
      assertTrue(exception.message!!.contains("line"))
      consoleLog("✅ PASS: Missing comma - ${exception.message}")
    }

    @Test
    @Order(4)
    fun `throw on duplicate key`() {
      val json = """{"dup":1, "dup":2}"""
      val exception = assertThrows<JsonParser.JsonException> { JsonParser.parseMap(json) }
      assertTrue(exception.message!!.contains("line"))
      consoleLog("✅ PASS: Duplicate key - ${exception.message}")
    }

    @Test
    @Order(5)
    fun `throw on unclosed array`() {
      val json = "[1,2,3"
      val exception = assertThrows<JsonParser.JsonException> { JsonParser.parse(json) }
      assertTrue(exception.message!!.contains("line"))
      consoleLog("✅ PASS: Unclosed array - ${exception.message}")
    }

    @Test
    @Order(6)
    fun `throw on leading zero`() {
      val json = """{"num": 012}"""
      val exception = assertThrows<JsonParser.JsonException> { JsonParser.parse(json) }
      assertTrue(exception.message!!.contains("line"))
      consoleLog("✅ PASS: Leading zero - ${exception.message}")
    }

    @Test
    @Order(7)
    fun `throw on control character`() {
      val json = "{\"control\": \"text\u0000\"}"
      val exception = assertThrows<JsonParser.JsonException> { JsonParser.parse(json) }
      assertTrue(exception.message!!.contains("line"))
      consoleLog("✅ PASS: Control character - ${exception.message}")
    }

    @Test
    @Order(8)
    fun `throw on extra bracket`() {
      val json = "[1,2,3,]]"
      val exception = assertThrows<JsonParser.JsonException> { JsonParser.parse(json) }
      assertTrue(exception.message!!.contains("line"))
      consoleLog("✅ PASS: Extra bracket - ${exception.message}")
    }

    @Test
    @Order(9)
    fun `throw on excess nesting`() {
      val deepNested = buildString {
        append("{")
        repeat(600) { append("\"level$it\":{") }
        append("42")
        repeat(600) { append("}") }
        append("}")
      }
      val exception = assertThrows<JsonParser.JsonException> { JsonParser.parse(deepNested) }
      assertTrue(exception.message!!.contains("line"))
      consoleLog("✅ PASS: Excess nesting - ${exception.message}")
    }

    @Test
    @Order(10)
    fun `throw on missing value in second line`() {
      val json =
        """
        {
          "ok": 1,
          "broken":
        }
        """
          .trimIndent()
      val ex = assertThrows<JsonParser.JsonException> { JsonParser.parse(json) }
      assertTrue(ex.message!!.contains("line"), "Should contain correct line number info")
      assertTrue(
        ex.message!!.contains("column") || ex.message!!.contains("col"),
        "Should contain column info",
      )
      consoleLog("✅ PASS: Missing value multi-line - ${ex.message}")
    }

    @Test
    @Order(11)
    fun `throw on array missing comma new line`() {
      val json =
        """
        [1
         2]
        """
          .trimIndent()
      val ex = assertThrows<JsonParser.JsonException> { JsonParser.parse(json) }
      assertTrue(ex.message!!.contains("line 2"))
      consoleLog("✅ PASS: Missing comma in array multi-line - ${ex.message}")
    }

    @Test
    @Order(12)
    fun `throw on unterminated string spans multiple lines`() {
      val json =
        """
        {
          "key": "value
        """
          .trimIndent()
      val ex = assertThrows<JsonParser.JsonException> { JsonParser.parse(json) }
      assertTrue(ex.message!!.contains("line 2"))
      consoleLog("✅ PASS: Unterminated string multi-line - ${ex.message}")
    }

    @Test
    @Order(13)
    fun `throw on different newline types`() {
      val json = "{\r\n \"a\":1\r \"b\": }"
      val ex = assertThrows<JsonParser.JsonException> { JsonParser.parse(json) }
      assertTrue(ex.message!!.contains("line 3"))
      consoleLog("✅ PASS: Different newline types - ${ex.message}")
    }

    @Test
    @Order(14)
    fun `throw on deep nested error line`() {
      val json =
        """
        {
          "a": {
            "b": [
              1,
              2,
              {"c": }
            ]
          }
        }
        """
          .trimIndent()
      val ex = assertThrows<JsonParser.JsonException> { JsonParser.parse(json) }
      assertTrue(ex.message!!.contains("line"), "Should contain correct error line info")
      consoleLog("✅ PASS: Deep nested error line - ${ex.message}")
    }
  }

  @Nested
  @TestMethodOrder(MethodOrderer.OrderAnnotation::class)
  inner class EdgeCaseTests {

    @Test
    @Order(0)
    fun `edge case tests`() {
      consoleLog("===== Edge Case Tests =====")
    }

    @Test
    @Order(1)
    fun `parse empty string`() {
      val exception = assertThrows<JsonParser.JsonException> { JsonParser.parse("") }
      assertTrue(exception.message!!.contains("line"))
      consoleLog("✅ PASS: Empty string")
    }

    @Test
    @Order(2)
    fun `parse number boundaries`() {
      assertAll(
        { JsonParser.parse("${Int.MAX_VALUE}")?.let { assertEquals(Int.MAX_VALUE, it) } },
        { JsonParser.parse("${Int.MIN_VALUE}")?.let { assertEquals(Int.MIN_VALUE, it) } },
        { JsonParser.parse("${Long.MAX_VALUE}")?.let { assertEquals(Long.MAX_VALUE, it) } },
        { JsonParser.parse("${Long.MIN_VALUE}")?.let { assertEquals(Long.MIN_VALUE, it) } },
        { assertEquals(123.456, JsonParser.parse("123.456") as Double, 0.001) },
        { assertEquals(1.23e-4, JsonParser.parse("1.23e-4") as Double, 0.001) },
      )
      consoleLog("✅ PASS: Number boundaries")
    }

    @Test
    @Order(3)
    fun `handle surrogate pairs`() {
      val json = """"\uD83D\uDE00""""
      JsonParser.parse(json)?.let { assertEquals("😀", it) }
      consoleLog("✅ PASS: Surrogate pairs")
    }
  }
}
