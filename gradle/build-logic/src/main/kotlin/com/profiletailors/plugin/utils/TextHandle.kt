package com.profiletailors.plugin.utils

object TextHandle {

  /**
   * Convert string containing '-' or '_' to camelCase (only capitalize the next character after the
   * delimiter). E.g.: a-bb-cCc -> aBbCCc hello_world -> helloWorld a-b_c-d -> aBCD
   */
  fun toCamelCase(str: String): String {
    val sb = StringBuilder()
    var upperNext = false
    for (ch in str) {
      when (ch) {
        '-',
        '_' -> upperNext = true
        else -> {
          if (upperNext) {
            sb.append(ch.uppercaseChar())
            upperNext = false
          } else {
            sb.append(ch)
          }
        }
      }
    }
    return sb.toString().replaceFirstChar { it.lowercase() }
  }

  /**
   * Wraps the given text into lines of the specified width.
   *
   * @param text The text to wrap.
   * @param lineWidth The maximum width of each line. Defaults to 80.
   * @return The wrapped text.
   */
  fun wrapText(text: String, lineWidth: Int = 80): String {
    val words = text.split("\\s+".toRegex())
    val result = StringBuilder()
    var currentLine = StringBuilder()

    for (word in words) {
      // Check if adding the word exceeds the line width
      if (currentLine.isNotEmpty() && currentLine.length + word.length + 1 > lineWidth) {
        result.appendLine(currentLine)
        currentLine = StringBuilder()
      }

      // Add word to current line
      if (currentLine.isEmpty()) {
        currentLine.append(word)
      } else {
        currentLine.append(" ").append(word)
      }
    }

    // Add the last line
    if (currentLine.isNotEmpty()) {
      result.append(currentLine)
    }

    return result.toString()
  }
}
