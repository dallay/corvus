package com.profiletailors.corvus

class JVMPlatform : Platform {
  override val name: String = "Java ${System.getProperty("java.version")}"
  override val isMobile: Boolean = false
  override val bridgeAvailability: BridgeAvailability = BridgeAvailability.LOCAL_BRIDGE
}

actual fun getPlatform(): Platform = JVMPlatform()
