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
    ?: // Try to load persisted connection metadata
    run {
      val prefs = context.getSharedPreferences(MOBILE_RUNTIME_PREFS, Context.MODE_PRIVATE)
      val targetId = prefs.getString("corvus.mobile.runtime.target", null)
      if (targetId != null) {
        // Valid persisted connection exists - try to use AndroidRuntimeBridge
        try {
          AndroidRuntimeBridge()
        } catch (e: Exception) {
          // Fall back to fail-closed if bridge creation fails
          FailClosedRuntimeFacade(
            unavailableReason = "Runtime bridge unavailable: ${e.message}",
            environmentSupported = true,
            capabilities =
              RuntimeCapabilities(
                streamingResponses = false,
                resumableSessionList = false,
                approvalRequests = false,
              ),
          )
        }
      } else {
        // Client-first: Fail closed - no connection configured
        FailClosedRuntimeFacade(
          unavailableReason =
            "No runtime endpoint configured. Please configure a connection target first.",
          environmentSupported = true,
          capabilities =
            RuntimeCapabilities(
              streamingResponses = false,
              resumableSessionList = false,
              approvalRequests = false,
            ),
        )
      }
    }

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
