package com.profiletailors.corvus

enum class BridgeAvailability {
  LOCAL_BRIDGE,
  COMPANION_REQUIRED,
}

interface Platform {
  val name: String
  val isMobile: Boolean
  val bridgeAvailability: BridgeAvailability
}

expect fun getPlatform(): Platform
