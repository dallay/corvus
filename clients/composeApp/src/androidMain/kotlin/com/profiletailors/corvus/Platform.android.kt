package com.profiletailors.corvus

import android.os.Build

class AndroidPlatform : Platform {
  override val name: String = "Android ${Build.VERSION.SDK_INT}"
  override val isMobile: Boolean = true
  override val bridgeAvailability: BridgeAvailability = BridgeAvailability.LOCAL_BRIDGE
}

actual fun getPlatform(): Platform = AndroidPlatform()
