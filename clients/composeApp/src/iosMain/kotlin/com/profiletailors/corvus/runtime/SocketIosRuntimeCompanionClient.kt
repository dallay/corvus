package com.profiletailors.corvus.runtime

// Type alias to the actual platform-specific implementation
// The actual implementation is in iosArm64Main and iosSimulatorArm64Main
expect class SocketIosRuntimeCompanionClient(config: IosRuntimeCompanionConfig) :
  IosRuntimeCompanionClient

expect class IosRuntimeCompanionConfig(host: String, port: Int, timeoutMs: Long) {
  companion object {
    val DEFAULT_COMPANION_PORT: Int
  }

  val host: String
  val port: Int
  val timeoutMs: Long
}
