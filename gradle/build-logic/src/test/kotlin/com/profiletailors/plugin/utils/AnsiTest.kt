package com.profiletailors.plugin.utils

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test

class AnsiTest {

  @Test
  fun colorUsesProvidedAnsiCode() {
    val actual = Ansi.color("ok", Ansi.Color.RED.code)

    assertEquals("\u001B[31mok\u001B[0m", actual)
  }

  @Test
  fun colorUsesDefaultAnsiCodeWhenCodeNotProvided() {
    val actual = Ansi.color("ok")

    assertEquals("\u001B[39mok\u001B[0m", actual)
  }

  @Test
  fun colorBoolReturnsColoredTextWithoutAlignmentWhenDisabled() {
    val actual = Ansi.colorBool(bool = true, align = false)

    assertEquals("\u001B[32mtrue\u001B[0m", actual)
  }

  @Test
  fun colorBoolRendersFalseBranch() {
    val actual = Ansi.colorBool(bool = false, align = true)

    assertTrue(actual.contains("false"))
    assertTrue(actual.endsWith("\u001B[0m"))
  }
}
