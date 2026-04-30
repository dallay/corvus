package com.profiletailors.corvus

import androidx.compose.ui.window.ComposeUIViewController
import com.profiletailors.corvus.runtime.SocketIosRuntimeCompanionClient
import com.profiletailors.corvus.runtime.installIosRuntimeCompanionClient
import platform.UIKit.UIViewController

fun MainViewController(): UIViewController {
  installIosRuntimeCompanionClient(SocketIosRuntimeCompanionClient())
  return ComposeUIViewController {
    val platform = IOSPlatform()
    App(platformOverride = platform)
  }
}
