package com.profiletailors.corvus

import androidx.compose.ui.window.ComposeUIViewController
import com.profiletailors.corvus.runtime.IosRuntimeCompanionConfig
import com.profiletailors.corvus.runtime.SocketIosRuntimeCompanionClient
import com.profiletailors.corvus.runtime.installIosRuntimeCompanionClient
import platform.UIKit.UIViewController

fun MainViewController(): UIViewController {
  installIosRuntimeCompanionClient(
    SocketIosRuntimeCompanionClient(
      IosRuntimeCompanionConfig(
        host = "127.0.0.1",
        port = IosRuntimeCompanionConfig.DEFAULT_COMPANION_PORT,
        timeoutMs = 30_000L,
      )
    )
  )
  return ComposeUIViewController {
    val platform = IOSPlatform()
    App(platformOverride = platform)
  }
}
