package com.profiletailors.corvus.runtime

import android.content.Context
import androidx.compose.runtime.Composable
import androidx.compose.runtime.remember
import androidx.compose.ui.platform.LocalContext
import com.profiletailors.corvus.ui.chat.MobileBridgeSnapshot

private const val MOBILE_RUNTIME_PREFS = "corvus.mobile.runtime"

// Client-first: Android should NOT assume packaged executable by default.
// Instead, it requires explicit endpoint configuration or trusted companion setup.
// This implements the client-first onboarding model per the delta specs.
internal fun createAndroidRuntimeFacade(
  context: Context,
  initialBridgeSnapshot: MobileBridgeSnapshot?,
): RuntimeFacade =
  initialBridgeSnapshot?.let(::PreviewMobileRuntimeFacade)
    ?: // Client-first: Fail closed - no default local runtime assumption
    FailClosedRuntimeFacade(
      unavailableReason =
        "No runtime endpoint configured. Please configure a connection target first.",
      environmentSupported = true,
      capabilities =
        RuntimeCapabilities(
          streamingResponses = false,
          resumableSessionList = true,
          approvalRequests = true,
        ),
    )

// Android supported connection methods for client-first model
internal val androidSupportedConnectionMethods: Set<RuntimeConnectionMethod> =
  setOf(RuntimeConnectionMethod.ENDPOINT_URL, RuntimeConnectionMethod.TRUSTED_COMPANION)

// Expected declaration for common code
actual val platformSupportedConnectionMethods: Set<RuntimeConnectionMethod>
  get() = androidSupportedConnectionMethods

@Composable
actual fun rememberPlatformRuntimeDependencies(
  initialBridgeSnapshot: MobileBridgeSnapshot?
): PlatformRuntimeDependencies {
  val context = LocalContext.current
  val facade =
    remember(context, initialBridgeSnapshot) {
      createAndroidRuntimeFacade(context, initialBridgeSnapshot)
    }
  val persistence =
    remember(context, initialBridgeSnapshot) {
      if (initialBridgeSnapshot != null) {
        InMemoryMobileRuntimePersistence()
      } else {
        AndroidPersistence(
          sharedPreferences =
            context.getSharedPreferences(MOBILE_RUNTIME_PREFS, Context.MODE_PRIVATE)
        )
      }
    }
  return remember(facade, persistence) { PlatformRuntimeDependencies(facade, persistence) }
}
