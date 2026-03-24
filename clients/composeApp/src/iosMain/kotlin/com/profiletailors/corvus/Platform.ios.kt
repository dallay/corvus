package com.profiletailors.corvus

import platform.UIKit.UIDevice

class IOSPlatform : Platform {
  override val name: String =
    UIDevice.currentDevice.systemName() + " " + UIDevice.currentDevice.systemVersion
  override val isMobile: Boolean = true
  override val bridgeAvailability: BridgeAvailability = BridgeAvailability.COMPANION_REQUIRED
}

actual fun getPlatform(): Platform = IOSPlatform()
