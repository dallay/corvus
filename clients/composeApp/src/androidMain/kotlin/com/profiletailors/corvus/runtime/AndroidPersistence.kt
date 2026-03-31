package com.profiletailors.corvus.runtime

import android.content.SharedPreferences

class AndroidPersistence(private val sharedPreferences: SharedPreferences) :
  MobileRuntimePersistence {
  override fun readLinkedRuntimeMetadata(): LinkedRuntimeMetadata? {
    val targetId = sharedPreferences.getString(KEY_TARGET_ID, null) ?: return null
    val transportStr = sharedPreferences.getString(KEY_TRANSPORT_MODE, null)
    val trustStr = sharedPreferences.getString(KEY_TRUST_MODE, null)
    val transportMode = transportStr.toTransportMode() ?: return null
    val trustMode = trustStr.toTrustMode() ?: return null
    val linkedAtEpochMs =
      sharedPreferences.getLong(KEY_LINKED_AT_EPOCH_MS, MISSING_LINKED_AT).takeIf {
        it != MISSING_LINKED_AT
      } ?: return null
    return LinkedRuntimeMetadata(
      targetId = targetId,
      transportMode = transportMode,
      trustMode = trustMode,
      linkedAtEpochMs = linkedAtEpochMs,
    )
  }

  override fun saveLinkedRuntimeMetadata(metadata: LinkedRuntimeMetadata) {
    sharedPreferences
      .edit()
      .putString(KEY_TARGET_ID, metadata.targetId)
      .putString(KEY_TRANSPORT_MODE, metadata.transportMode.name)
      .putString(KEY_TRUST_MODE, metadata.trustMode.name)
      .apply {
        metadata.linkedAtEpochMs?.let { putLong(KEY_LINKED_AT_EPOCH_MS, it) }
          ?: remove(KEY_LINKED_AT_EPOCH_MS)
      }
      .apply()
  }

  override fun clearLinkedRuntimeMetadata() {
    sharedPreferences
      .edit()
      .remove(KEY_TARGET_ID)
      .remove(KEY_TRANSPORT_MODE)
      .remove(KEY_TRUST_MODE)
      .remove(KEY_LINKED_AT_EPOCH_MS)
      .apply()
  }

  override fun readActiveSessionId(): RuntimeSessionId? =
    sharedPreferences.getString(KEY_ACTIVE_SESSION_ID, null)?.let(::RuntimeSessionId)

  override fun saveActiveSessionId(sessionId: RuntimeSessionId) {
    sharedPreferences.edit().putString(KEY_ACTIVE_SESSION_ID, sessionId.value).apply()
  }

  override fun clearActiveSessionId() {
    sharedPreferences.edit().remove(KEY_ACTIVE_SESSION_ID).apply()
  }

  private companion object {
    private const val KEY_TARGET_ID = "corvus.mobile.runtime.target"
    private const val KEY_TRANSPORT_MODE = "corvus.mobile.runtime.transport"
    private const val KEY_TRUST_MODE = "corvus.mobile.runtime.trust"
    private const val KEY_LINKED_AT_EPOCH_MS = "corvus.mobile.runtime.linkedAtEpochMs"
    private const val KEY_ACTIVE_SESSION_ID = "corvus.mobile.runtime.activeSessionId"
    private const val MISSING_LINKED_AT = Long.MIN_VALUE
  }
}

private fun String?.toTransportMode(): RuntimeTransportMode? =
  when (this) {
    RuntimeTransportMode.CLI_BRIDGE.name -> RuntimeTransportMode.CLI_BRIDGE
    RuntimeTransportMode.DIRECT.name -> RuntimeTransportMode.DIRECT
    RuntimeTransportMode.HTTP_GATEWAY.name -> RuntimeTransportMode.HTTP_GATEWAY
    else -> null
  }

private fun String?.toTrustMode(): RuntimeTrustMode? =
  when (this) {
    RuntimeTrustMode.BRIDGE_LINKED.name -> RuntimeTrustMode.BRIDGE_LINKED
    RuntimeTrustMode.HOST_TRUSTED.name -> RuntimeTrustMode.HOST_TRUSTED
    RuntimeTrustMode.HTTP_PAIRED.name -> RuntimeTrustMode.HTTP_PAIRED
    else -> null
  }
