package com.profiletailors.corvus.runtime

import com.profiletailors.corvus.ui.chat.MobileBridgeSnapshot
import kotlin.test.Test
import kotlin.test.assertIs

class PlatformRuntimeDependenciesJvmTest {
  @Test
  fun `should use shared rust cli bridge when preview mode is disabled`() {
    assertIs<JvmRuntimeFacade>(createJvmRuntimeFacade(initialBridgeSnapshot = null))
    assertIs<InMemoryMobileRuntimePersistence>(createJvmRuntimePersistence())
  }

  @Test
  fun `should keep preview facade when a bridge snapshot is injected`() {
    assertIs<PreviewMobileRuntimeFacade>(
      createJvmRuntimeFacade(
        initialBridgeSnapshot =
          MobileBridgeSnapshot(
            runtimeAvailable = true,
            linkEstablished = true,
            sessionCapable = true,
          )
      )
    )
  }
}
