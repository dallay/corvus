package com.profiletailors.plugin.spotless

import com.diffplug.spotless.FormatterFunc
import com.diffplug.spotless.FormatterStep
import java.io.Serializable

/** disable regular expression step */
class DisableRegularExpressionStep {
  companion object {
    fun create(regExpList: List<Regex> = listOf()): FormatterStep {
      val state = State(regExpList.map { it.pattern })

      return FormatterStep.create("DisableRegularExpressionStep", state) { s ->
        FormatterFunc { unixStr ->
          val regex = s.patterns.firstOrNull { Regex(it).containsMatchIn(unixStr) }
          if (regex != null) {
            val re = Regex(regex)
            val match = re.find(unixStr)
            val lineNumber =
              match?.let { unixStr.take(it.range.first).count { c -> c == '\n' } + 1 } ?: -1
            val snippet = match?.value ?: unixStr.take(200)

            throw AssertionError(
              buildString {
                  appendLine("[${regex}]L$lineNumber")
                  appendLine(snippet)
                }
                .trim()
            )
          }
          unixStr
        }
      }
    }
  }

  data class State(val patterns: List<String>) : Serializable {
    companion object {
      private const val serialVersionUID = 1L
    }
  }
}
