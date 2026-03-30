package com.profiletailors.corvus

import androidx.compose.ui.window.ComposeUIViewController

fun MainViewController() = ComposeUIViewController {
  val platform = IOSPlatform()
  App(platformOverride = platform)
}
