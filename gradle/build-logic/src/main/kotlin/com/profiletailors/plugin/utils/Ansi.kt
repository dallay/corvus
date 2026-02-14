package com.profiletailors.plugin.utils

/** ANSI Console Utility Class */
object Ansi {

  enum class Color(val code: String) {
    RESET("0"),
    BLACK("30"),
    RED("31"),
    GREEN("32"),
    YELLOW("33"),
    BLUE("34"),
    MAGENTA("35"),
    CYAN("36"),
    WHITE("37"),
    DEFAULT("39"),
    BACKGROUND_BLACK("40"),
    BACKGROUND_RED("41"),
    BACKGROUND_GREEN("42"),
    BACKGROUND_YELLOW("43"),
    BACKGROUND_BLUE("44"),
    BACKGROUND_MAGENTA("45"),
    BACKGROUND_CYAN("46"),
    BACKGROUND_WHITE("47"),
    BRIGHT_BLACK("90"),
    BRIGHT_RED("91"),
    BRIGHT_GREEN("92"),
    BRIGHT_YELLOW("93"),
    BRIGHT_BLUE("94"),
    BRIGHT_MAGENTA("95"),
    BRIGHT_CYAN("96"),
    BRIGHT_WHITE("97"),
  }

  /**
   * Color print
   *
   * @param str String
   * @param colorCode Color code
   */
  fun color(str: String, colorCode: String? = "39"): String {
    return "${colorCode!!.toAnsiCode()}$str${Color.RESET.code.toAnsiCode()}"
  }

  fun colorBool(bool: Boolean, align: Boolean = true): String {
    val text =
      when (bool) {
        true -> color("true", Color.GREEN.code)
        false -> color("false", Color.RED.code)
      }
    return if (align) text.padStart(5) else text
  }

  private fun String.toAnsiCode() = "\u001B[${this}m"
}
