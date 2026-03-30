package com.profiletailors.corvus.runtime

import platform.Foundation.NSUserDefaults

class IosPersistence(private val defaults: NSUserDefaults = NSUserDefaults.standardUserDefaults) :
  MobileRuntimePersistence {
  override fun readLinkedRuntimeMetadata(): LinkedRuntimeMetadata? {
    val targetId = defaults.stringForKey(KEY_TARGET_ID) ?: return null
    return LinkedRuntimeMetadata(
      targetId = targetId,
      transportMode = defaults.stringForKey(KEY_TRANSPORT_MODE).toTransportMode(),
      trustMode = defaults.stringForKey(KEY_TRUST_MODE).toTrustMode(),
      linkedAtEpochMs = defaults.objectForKey(KEY_LINKED_AT_EPOCH_MS) as? Long,
    )
  }

  override fun saveLinkedRuntimeMetadata(metadata: LinkedRuntimeMetadata) {
    defaults.setObject(metadata.targetId, KEY_TARGET_ID)
    defaults.setObject(metadata.transportMode.name, KEY_TRANSPORT_MODE)
    defaults.setObject(metadata.trustMode.name, KEY_TRUST_MODE)
    metadata.linkedAtEpochMs?.let { defaults.setObject(it, KEY_LINKED_AT_EPOCH_MS) }
      ?: defaults.removeObjectForKey(KEY_LINKED_AT_EPOCH_MS)
  }

  override fun clearLinkedRuntimeMetadata() {
    defaults.removeObjectForKey(KEY_TARGET_ID)
    defaults.removeObjectForKey(KEY_TRANSPORT_MODE)
    defaults.removeObjectForKey(KEY_TRUST_MODE)
    defaults.removeObjectForKey(KEY_LINKED_AT_EPOCH_MS)
  }

  override fun readActiveSessionId(): RuntimeSessionId? =
    defaults.stringForKey(KEY_ACTIVE_SESSION_ID)?.let(::RuntimeSessionId)

  override fun saveActiveSessionId(sessionId: RuntimeSessionId) {
    defaults.setObject(sessionId.value, KEY_ACTIVE_SESSION_ID)
  }

  override fun clearActiveSessionId() {
    defaults.removeObjectForKey(KEY_ACTIVE_SESSION_ID)
  }

  private companion object {
    private const val KEY_TARGET_ID = "corvus.mobile.runtime.target"
    private const val KEY_TRANSPORT_MODE = "corvus.mobile.runtime.transport"
    private const val KEY_TRUST_MODE = "corvus.mobile.runtime.trust"
    private const val KEY_LINKED_AT_EPOCH_MS = "corvus.mobile.runtime.linkedAtEpochMs"
    private const val KEY_ACTIVE_SESSION_ID = "corvus.mobile.runtime.activeSessionId"
  }
}

private fun String?.toTransportMode(): RuntimeTransportMode =
  when (this) {
    RuntimeTransportMode.CLI_BRIDGE.name,
    "CLI bridge",
    null -> RuntimeTransportMode.CLI_BRIDGE
    RuntimeTransportMode.DIRECT.name -> RuntimeTransportMode.DIRECT
    RuntimeTransportMode.HTTP_GATEWAY.name -> RuntimeTransportMode.HTTP_GATEWAY
    else -> RuntimeTransportMode.CLI_BRIDGE
  }

private fun String?.toTrustMode(): RuntimeTrustMode =
  when (this) {
    RuntimeTrustMode.BRIDGE_LINKED.name,
    "Bridge linked",
    null -> RuntimeTrustMode.BRIDGE_LINKED
    RuntimeTrustMode.HOST_TRUSTED.name -> RuntimeTrustMode.HOST_TRUSTED
    RuntimeTrustMode.HTTP_PAIRED.name -> RuntimeTrustMode.HTTP_PAIRED
    else -> RuntimeTrustMode.BRIDGE_LINKED
  }
