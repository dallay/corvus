package com.profiletailors.corvus.runtime

import androidx.compose.runtime.Composable
import com.profiletailors.corvus.ui.chat.MobileBridgeSnapshot

data class PlatformRuntimeDependencies(
  val facade: MobileRuntimeFacade,
  val persistence: MobileRuntimePersistence,
)

internal fun createPreviewAwareRuntimeFacade(
  initialBridgeSnapshot: MobileBridgeSnapshot?,
  runtimeFactory: () -> RuntimeFacade,
): RuntimeFacade = initialBridgeSnapshot?.let(::PreviewMobileRuntimeFacade) ?: runtimeFactory()

// Expected platform-specific supported connection methods
expect val platformSupportedConnectionMethods: Set<RuntimeConnectionMethod>

@Composable
expect fun rememberPlatformRuntimeDependencies(
  initialBridgeSnapshot: MobileBridgeSnapshot? = null
): PlatformRuntimeDependencies
