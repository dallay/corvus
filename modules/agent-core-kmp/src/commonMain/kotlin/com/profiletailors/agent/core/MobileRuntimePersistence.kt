package com.profiletailors.agent.core

interface MobileRuntimePersistence {
  fun readLinkedRuntimeMetadata(): LinkedRuntimeMetadata?

  fun saveLinkedRuntimeMetadata(metadata: LinkedRuntimeMetadata)

  fun clearLinkedRuntimeMetadata()

  fun readActiveSessionId(): SessionId?

  fun saveActiveSessionId(sessionId: SessionId)

  fun clearActiveSessionId()
}
