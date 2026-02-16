package com.profiletailors.corvus

import platform.UIKit.UIDevice

class IOSPlatform : Platform {
  override val name: String =
    UIDevice.currentDevice.systemName() + " " + UIDevice.currentDevice.systemVersion
  override val isMobile: Boolean = true
}

actual fun getPlatform(): Platform = IOSPlatform()
