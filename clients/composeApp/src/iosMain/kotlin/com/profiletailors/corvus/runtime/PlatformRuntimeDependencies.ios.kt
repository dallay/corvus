package com.profiletailors.corvus.runtime

import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import com.profiletailors.corvus.ui.chat.MobileBridgeSnapshot

private val defaultIosRuntimeCompanionClient = MissingInfrastructureIosRuntimeCompanionClient()

// Client-first: iOS already fails closed when no companion is configured.
// This is the correct client-first behavior per the delta specs.
internal fun createIosRuntimeFacade(
  initialBridgeSnapshot: MobileBridgeSnapshot?,
  companionClientProvider: IosRuntimeCompanionClientProvider =
    RegisteredIosRuntimeCompanionClientProvider,
): RuntimeFacade =
  createPreviewAwareRuntimeFacade(initialBridgeSnapshot) {
    IosRuntimeBridge(companionClientProvider.current() ?: defaultIosRuntimeCompanionClient)
  }

// iOS supported connection methods for client-first model
internal val iosSupportedConnectionMethods: Set<RuntimeConnectionMethod> =
  setOf(RuntimeConnectionMethod.TRUSTED_COMPANION, RuntimeConnectionMethod.ENDPOINT_URL)

// Expected declaration for common code
actual val platformSupportedConnectionMethods: Set<RuntimeConnectionMethod>
  get() = iosSupportedConnectionMethods

@Composable
actual fun rememberPlatformRuntimeDependencies(
  initialBridgeSnapshot: MobileBridgeSnapshot?
): PlatformRuntimeDependencies {
  val facade = remember(initialBridgeSnapshot) { createIosRuntimeFacade(initialBridgeSnapshot) }
  val persistence =
    remember(initialBridgeSnapshot) {
      if (initialBridgeSnapshot != null) {
        InMemoryMobileRuntimePersistence()
      } else {
        IosPersistence()
      }
    }
  return remember(facade, persistence) { PlatformRuntimeDependencies(facade, persistence) }
}
