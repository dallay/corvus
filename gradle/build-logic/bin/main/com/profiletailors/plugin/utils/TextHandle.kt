package com.profiletailors.plugin.utils

object TextHandle {

  /**
   * 将包含 '-' 或 '_' 的字符串按规则转换为小驼峰式（仅在分隔符后将下一个字符大写）。 例如： a-bb-cCc -> aBbCCc hello_world -> helloWorld
   * a-b_c-d -> aBCD
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
      // 检查添加单词后是否超过行宽
      if (currentLine.isNotEmpty() && currentLine.length + word.length + 1 > lineWidth) {
        result.appendLine(currentLine)
        currentLine = StringBuilder()
      }

      // 添加单词到当前行
      if (currentLine.isEmpty()) {
        currentLine.append(word)
      } else {
        currentLine.append(" ").append(word)
      }
    }

    // 添加最后一行
    if (currentLine.isNotEmpty()) {
      result.append(currentLine)
    }

    return result.toString()
  }
}
