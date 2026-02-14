package com.profiletailors.plugin.utils

import java.math.BigDecimal

@Suppress("detekt:TooManyFunctions")
object JsonParser {
  const val MAX_DEPTH = 512

  enum class NumberMode {
    PERFORMANCE,
    PRECISE,
  }

  var numberMode: NumberMode = NumberMode.PERFORMANCE

  @Suppress("UNCHECKED_CAST")
  fun parseMap(jsonString: String): Map<String, Any?> {
    val result = parse(jsonString)
    if (result !is Map<*, *>) {
      throw JsonException("Root value is not a JSON object", 0, 1, 1)
    }
    return result as Map<String, Any?>
  }

  @Suppress("UNCHECKED_CAST")
  fun parseList(jsonString: String): List<Any?> {
    val result = parse(jsonString)
    if (result !is List<*>) {
      throw JsonException("Root value is not a JSON array", 0, 1, 1)
    }
    return result
  }

  fun parse(jsonString: String): Any? {
    return Parser(jsonString.trim()).parseValue(0)
  }

  private class Parser(json: String) {
    private val chars: CharArray = json.toCharArray()
    private val length = chars.size
    private var index = 0
    private var line = 1
    private var col = 1

    fun parseValue(depth: Int): Any? {
      if (depth > MAX_DEPTH) fail("Exceeded maximum nesting depth")
      skipWhitespace()
      if (index >= length) fail("Unexpected end of JSON")

      return when (val c = chars[index]) {
        '{' -> parseObject(depth + 1)
        '[' -> parseArray(depth + 1)
        '"' -> parseString()
        in '0'..'9',
        '-' -> parseNumber()
        't',
        'f' -> parseBoolean()
        'n' -> parseNull()
        else -> fail("Unexpected character '$c'")
      }
    }

    private fun skipWhitespace() {
      while (index < length && chars[index].isJsonWhitespace()) advance()
    }

    private fun advance() {
      if (index < length) {
        when (chars[index]) {
          '\r' -> {
            line++
            col = 1
            index++
            if (index < length && chars[index] == '\n') index++
          }
          '\n' -> {
            line++
            col = 1
            index++
          }
          else -> {
            col++
            index++
          }
        }
      }
    }

    private fun parseString(): String {
      val sb = StringBuilder()
      advance() // skip "
      val startPos = index
      while (index < length) {
        val c = chars[index]
        advance()
        when (c) {
          '"' -> return sb.toString()
          '\\' -> handleEscapeSequence(sb)
          else -> {
            if (c.code < 0x20) fail("Unescaped control character: 0x${c.code.toString(16)}")
            sb.append(c)
          }
        }
      }
      throw JsonException("Unterminated string", startPos, line, col)
    }

    private fun handleEscapeSequence(sb: StringBuilder) {
      if (index >= length) fail("Unterminated escape sequence")
      val esc = chars[index]
      advance()
      when (esc) {
        '"',
        '\\',
        '/' -> sb.append(esc)
        'b' -> sb.append('\b')
        'f' -> sb.append('\u000C')
        'n' -> sb.append('\n')
        'r' -> sb.append('\r')
        't' -> sb.append('\t')
        'u' -> parseUnicodeEscape(sb)
        else -> fail("Invalid escape character '$esc'")
      }
    }

    private fun parseUnicodeEscape(sb: StringBuilder) {
      if (index + 4 > length) fail("Incomplete unicode escape")
      val hex = String(chars, index, 4)
      index += 4
      col += 4
      try {
        when (val codePoint = hex.toInt(16)) {
          in 0xD800..0xDBFF -> {
            if (index + 6 > length || chars[index] != '\\' || chars[index + 1] != 'u') {
              fail("Missing low surrogate")
            }
            index += 2
            col += 2
            val lowHex = String(chars, index, 4)
            val lowCode = lowHex.toInt(16)
            if (lowCode !in 0xDC00..0xDFFF) {
              fail("Invalid low surrogate: \\u$lowHex")
            }
            index += 4
            col += 4
            val fullCode = 0x10000 + (codePoint - 0xD800) * 0x400 + (lowCode - 0xDC00)
            sb.append(Character.toChars(fullCode))
          }
          in 0xDC00..0xDFFF -> {
            fail("Unexpected low surrogate without preceding high surrogate")
          }
          else -> {
            sb.append(Character.toChars(codePoint))
          }
        }
      } catch (_: NumberFormatException) {
        fail("Invalid unicode escape '\\u$hex'")
      }
    }

    private fun parseObject(depth: Int): Map<String, Any?> {
      val map = LinkedHashMap<String, Any?>()
      advance() // skip {
      skipWhitespace()
      if (consumeIf('}')) return map
      while (true) {
        skipWhitespace()
        if (chars.getOrNull(index) != '"') fail("Object keys must be in double quotes")
        val key = parseString()
        if (map.containsKey(key)) fail("Duplicate key '$key'")
        skipWhitespace()
        if (!consumeIf(':')) fail("Expected ':' after key")
        val value = parseValue(depth)
        map[key] = value
        skipWhitespace()
        if (consumeIf('}')) return map
        if (!consumeIf(',')) fail("Expected ',' or '}'")
        skipWhitespace()
        if (chars.getOrNull(index) == '}') fail("Trailing comma in object")
      }
    }

    private fun parseArray(depth: Int): List<Any?> {
      val list = ArrayList<Any?>()
      advance() // skip [
      skipWhitespace()
      if (consumeIf(']')) return list
      while (true) {
        skipWhitespace()
        list.add(parseValue(depth))
        skipWhitespace()
        if (consumeIf(']')) return list
        if (!consumeIf(',')) fail("Expected ',' or ']'")
        skipWhitespace()
        if (chars.getOrNull(index) == ']') fail("Trailing comma in array")
      }
    }

    @Suppress("detekt:CyclomaticComplexMethod", "detekt:NestedBlockDepth")
    private fun parseNumber(): Number {
      val start = index
      var hasExponent = false
      var hasDecimal = false

      if (chars[index] == '-') advance()

      if (index < length && chars[index] == '0') {
        advance()
        if (index < length && chars[index] in '0'..'9') fail("Leading zeros not allowed")
      } else {
        while (index < length && chars[index] in '0'..'9') advance()
      }

      if (index < length && chars[index] == '.') {
        hasDecimal = true
        advance()
        if (index >= length || chars[index] !in '0'..'9') fail("Expected digit after decimal point")
        while (index < length && chars[index] in '0'..'9') advance()
      }

      if (index < length && chars[index] in "eE") {
        hasExponent = true
        advance()
        if (index < length && chars[index] in "+-") advance()
        if (index >= length || chars[index] !in '0'..'9') fail("Expected digit in exponent")
        while (index < length && chars[index] in '0'..'9') advance()
      }

      val numStr = String(chars, start, index - start)
      return try {
        when (numberMode) {
          NumberMode.PRECISE -> BigDecimal(numStr)
          NumberMode.PERFORMANCE ->
            when {
              hasDecimal || hasExponent -> numStr.toDouble()
              numStr.toLongOrNull() != null -> {
                val l = numStr.toLong()
                if (l in Int.MIN_VALUE..Int.MAX_VALUE) l.toInt() else l
              }
              else -> BigDecimal(numStr)
            }
        }
      } catch (_: NumberFormatException) {
        fail("Invalid number format: '$numStr'", start)
      }
    }

    private fun parseBoolean(): Boolean {
      return when {
        matchKeyword("true") -> true
        matchKeyword("false") -> false
        else -> fail("Expected boolean value")
      }
    }

    private fun parseNull(): Any? {
      if (matchKeyword("null")) return null
      fail("Expected null value")
    }

    private fun matchKeyword(keyword: String): Boolean {
      if (
        index + keyword.length <= length &&
          String(chars).regionMatches(index, keyword, 0, keyword.length)
      ) {
        repeat(keyword.length) { advance() }
        return true
      }
      return false
    }

    private fun Char.isJsonWhitespace() = this in " \t\n\r"

    private fun consumeIf(expected: Char): Boolean {
      if (index < length && chars[index] == expected) {
        advance()
        return true
      }
      return false
    }

    private fun fail(msg: String, pos: Int = index): Nothing {
      throw JsonException(msg, pos, line, col)
    }
  }

  class JsonException(message: String, position: Int, line: Int, column: Int) :
    RuntimeException("$message at line $line, column $column (index $position)")
}
