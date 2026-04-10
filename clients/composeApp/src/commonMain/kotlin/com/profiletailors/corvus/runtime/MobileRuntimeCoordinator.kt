package com.profiletailors.corvus.runtime

import com.profiletailors.corvus.ui.chat.ChatMessage
import com.profiletailors.corvus.ui.chat.ChatRole
import com.profiletailors.corvus.ui.chat.MobileBridgeSnapshot
import com.profiletailors.corvus.ui.chat.MobileRecoveryKind
import kotlin.math.absoluteValue

private const val DEFAULT_TARGET_ID = "mobile-runtime"
private const val PREVIEW_SESSION_PAD = 12

data class MobileRuntimeCoordinatorState(
  val bridgeSnapshot: MobileBridgeSnapshot,
  val resumableSessions: List<RuntimeSession> = emptyList(),
  val activeSessionId: RuntimeSessionId? = null,
  val messages: List<ChatMessage> = emptyList(),
  val pendingApproval: RuntimeApprovalRequest? = null,
  val targetLabel: String? = null,
) {
  val isChatReady: Boolean =
    bridgeSnapshot.toOnboardingState().status ==
      com.profiletailors.corvus.ui.chat.MobileOnboardingStatus.SESSION_READY
}

class MobileRuntimeCoordinator(
  private val facade: RuntimeFacade,
  private val persistence: MobileRuntimePersistence,
) {
  var state: MobileRuntimeCoordinatorState =
    MobileRuntimeCoordinatorState(
      bridgeSnapshot =
        MobileBridgeSnapshot(
          runtimeAvailable = false,
          linkEstablished = false,
          sessionCapable = false,
        )
    )
    private set

  fun refresh() {
    val readiness = facade.probeReadiness()
    val linkedMetadata = persistence.readLinkedRuntimeMetadata()
    val persistedSessionId = persistence.readActiveSessionId()
    val canLoadResumableSessions =
      readiness.runtimeAvailable &&
        readiness.linkEstablished &&
        readiness.sessionCapable &&
        (readiness.capabilities.resumableSessionList || facade.capabilities.resumableSessionList)
    val sessionLookup =
      if (canLoadResumableSessions) {
        runCatching { facade.listResumableSessions() }
      } else {
        Result.success(emptyList())
      }
    val sessions = sessionLookup.getOrDefault(emptyList())

    val activeSessionId = computeActiveSessionId(persistedSessionId, sessions, readiness)
    val recoveryOverride =
      computeRecoveryOverride(
        persistedSessionId,
        activeSessionId,
        sessionLookup,
        linkedMetadata,
        readiness,
      )

    if (readiness.linkEstablished && linkedMetadata == null) {
      persistence.saveLinkedRuntimeMetadata(LinkedRuntimeMetadata(targetId = DEFAULT_TARGET_ID))
    }

    val updatedLinkedMetadata = persistence.readLinkedRuntimeMetadata()
    state =
      state.copy(
        bridgeSnapshot =
          MobileBridgeSnapshot(
            runtimeAvailable = readiness.runtimeAvailable,
            linkEstablished = readiness.linkEstablished,
            sessionCapable = readiness.sessionCapable,
            sessionId = activeSessionId?.value,
            environmentSupported = readiness.environmentSupported,
            recoveryOverride = recoveryOverride,
            targetLabel = updatedLinkedMetadata?.targetId,
          ),
        resumableSessions = sessions,
        activeSessionId = activeSessionId,
        pendingApproval = null,
        targetLabel = updatedLinkedMetadata?.targetId,
      )
  }

  private fun computeActiveSessionId(
    persistedSessionId: RuntimeSessionId?,
    sessions: List<RuntimeSession>,
    readiness: RuntimeReadinessSnapshot,
  ): RuntimeSessionId? {
    return when {
      persistedSessionId == null && readiness.activeSessionId != null -> {
        val runtimeSessionId = readiness.activeSessionId
        persistence.saveActiveSessionId(runtimeSessionId)
        runtimeSessionId
      }

      persistedSessionId != null &&
        (sessions.any { it.id == persistedSessionId } ||
          readiness.activeSessionId == persistedSessionId) -> {
        persistedSessionId
      }

      else -> null
    }
  }

  private fun computeRecoveryOverride(
    persistedSessionId: RuntimeSessionId?,
    activeSessionId: RuntimeSessionId?,
    sessionLookup: Result<List<RuntimeSession>>,
    linkedMetadata: LinkedRuntimeMetadata?,
    readiness: RuntimeReadinessSnapshot,
  ): MobileRecoveryKind? {
    return when {
      // Only clear persisted ID if session lookup succeeded and the persisted ID is truly absent
      persistedSessionId != null && activeSessionId == null && sessionLookup.isSuccess -> {
        persistence.clearActiveSessionId()
        MobileRecoveryKind.SESSION_UNAVAILABLE
      }

      sessionLookup.isFailure && readiness.linkEstablished ->
        MobileRecoveryKind.LINKED_BUT_NOT_SESSION_READY

      linkedMetadata != null && !readiness.linkEstablished && !readiness.runtimeAvailable ->
        MobileRecoveryKind.TRANSPORT_UNAVAILABLE

      else -> null
    }
  }

  fun startNewSession() {
    runCatching { facade.createSession() }
      .onSuccess { session ->
        persistence.saveActiveSessionId(session.id)
        state =
          state.copy(
            bridgeSnapshot =
              state.bridgeSnapshot.copy(sessionId = session.id.value, recoveryOverride = null),
            activeSessionId = session.id,
            resumableSessions =
              (state.resumableSessions -
                state.resumableSessions.filter { it.id == session.id }.toSet()) + session,
            messages = emptyList(),
            pendingApproval = null,
          )
      }
      .onFailure { refresh() }
  }

  fun resumeSession(sessionId: RuntimeSessionId) {
    runCatching { facade.resumeSession(sessionId) }
      .onSuccess { session ->
        persistence.saveActiveSessionId(session.id)
        state =
          state.copy(
            bridgeSnapshot =
              state.bridgeSnapshot.copy(sessionId = session.id.value, recoveryOverride = null),
            activeSessionId = session.id,
            resumableSessions =
              state.resumableSessions.map { if (it.id == session.id) session else it },
            pendingApproval = null,
          )
      }
      .onFailure { refresh() }
  }

  fun endActiveSession() {
    val activeSessionId = state.activeSessionId ?: return
    runCatching { facade.endSession(activeSessionId) }
      .onSuccess {
        persistence.clearActiveSessionId()
        state =
          state.copy(
            bridgeSnapshot = state.bridgeSnapshot.copy(sessionId = null, recoveryOverride = null),
            activeSessionId = null,
            messages = emptyList(),
            pendingApproval = null,
          )
      }
      .onFailure { refresh() }
  }

  fun sendMessage(prompt: String) {
    val activeSessionId = state.activeSessionId ?: return
    val userMessage = ChatMessage(id = nextMessageId(), role = ChatRole.User, content = prompt)
    state = state.copy(messages = state.messages + userMessage)
    applyTurnResult(facade.sendMessage(activeSessionId, prompt))
  }

  fun submitApproval(decision: RuntimeApprovalDecision) {
    val approval = state.pendingApproval ?: return
    state = state.copy(pendingApproval = null)
    applyTurnResult(facade.submitApproval(approval.id, decision, approval.sessionId))
  }

  fun disconnect() {
    persistence.clearActiveSessionId()
    persistence.clearLinkedRuntimeMetadata()
    state =
      state.copy(
        bridgeSnapshot =
          MobileBridgeSnapshot(
            runtimeAvailable = state.bridgeSnapshot.runtimeAvailable,
            linkEstablished = false,
            sessionCapable = false,
            environmentSupported = state.bridgeSnapshot.environmentSupported,
          ),
        resumableSessions = emptyList(),
        activeSessionId = null,
        messages = emptyList(),
        pendingApproval = null,
        targetLabel = null,
      )
  }

  private fun applyTurnResult(result: RuntimeTurnResult) {
    val assistantMessages = mutableListOf<ChatMessage>()
    var pendingApproval: RuntimeApprovalRequest? = state.pendingApproval
    result.events.forEach { event ->
      when (event) {
        is RuntimeEvent.AssistantChunk ->
          assistantMessages += assistantChatMessage(event.text, assistantMessages.size)
        is RuntimeEvent.AssistantMessage ->
          assistantMessages += assistantChatMessage(event.text, assistantMessages.size)

        is RuntimeEvent.ApprovalPending -> pendingApproval = event.request
        is RuntimeEvent.Failure -> {
          assistantMessages +=
            ChatMessage(
              id = nextMessageId(assistantMessages.size),
              role = ChatRole.Assistant,
              content = event.message,
            )
        }
      }
    }
    state =
      state.copy(messages = state.messages + assistantMessages, pendingApproval = pendingApproval)
  }

  private fun assistantChatMessage(text: String, offset: Int): ChatMessage =
    ChatMessage(id = nextMessageId(offset), role = ChatRole.Assistant, content = text)

  private fun nextMessageId(offset: Int = 0): Int =
    computeNextMessageId(state.messages.size, offset)
}

// Top-level helper for message ID generation
private fun computeNextMessageId(currentSize: Int, offset: Int = 0): Int = currentSize + offset + 1

class InMemoryMobileRuntimePersistence(
  private var linkedRuntimeMetadata: LinkedRuntimeMetadata? = null,
  private var activeRuntimeSessionId: RuntimeSessionId? = null,
) : MobileRuntimePersistence {
  override fun readLinkedRuntimeMetadata(): LinkedRuntimeMetadata? = linkedRuntimeMetadata

  override fun saveLinkedRuntimeMetadata(metadata: LinkedRuntimeMetadata) {
    linkedRuntimeMetadata = metadata
  }

  override fun clearLinkedRuntimeMetadata() {
    linkedRuntimeMetadata = null
  }

  override fun readActiveSessionId(): RuntimeSessionId? = activeRuntimeSessionId

  override fun saveActiveSessionId(sessionId: RuntimeSessionId) {
    activeRuntimeSessionId = sessionId
  }

  override fun clearActiveSessionId() {
    activeRuntimeSessionId = null
  }
}

class PreviewMobileRuntimeFacade(initialSnapshot: MobileBridgeSnapshot) : MobileRuntimeFacade {
  private var readiness =
    initialSnapshot
      .toRuntimeReadinessSnapshot()
      .copy(
        capabilities =
          RuntimeCapabilities(
            streamingResponses = false,
            resumableSessionList = true,
            approvalRequests = true,
          )
      )
  private var nextSessionIndex = 1
  private val sessions = mutableListOf<RuntimeSession>()

  override val capabilities: RuntimeCapabilities =
    RuntimeCapabilities(
      streamingResponses = false,
      resumableSessionList = true,
      approvalRequests = true,
    )

  override fun probeReadiness(): RuntimeReadinessSnapshot =
    readiness.copy(capabilities = capabilities)

  override fun createSession(metadata: Map<String, String>): RuntimeSession {
    val session =
      RuntimeSession(
        id = previewSessionId(nextSessionIndex++),
        title = metadata["title"] ?: "Mobile session ${sessions.size + 1}",
        isActive = true,
      )
    sessions.removeAll { it.id == session.id }
    sessions += session
    readiness =
      readiness.copy(linkEstablished = true, sessionCapable = true, activeSessionId = session.id)
    return session
  }

  override fun listResumableSessions(): List<RuntimeSession> = sessions.toList()

  override fun resumeSession(sessionId: RuntimeSessionId): RuntimeSession {
    val session =
      sessions.firstOrNull { it.id == sessionId } ?: RuntimeSession(id = sessionId, isActive = true)
    sessions.removeAll { it.id == sessionId }
    sessions += session.copy(isActive = true)
    readiness =
      readiness.copy(linkEstablished = true, sessionCapable = true, activeSessionId = sessionId)
    return session.copy(isActive = true)
  }

  override fun endSession(sessionId: RuntimeSessionId) {
    val updatedSessions =
      sessions.map { session ->
        if (session.id == sessionId) session.copy(isActive = false) else session
      }
    sessions.clear()
    sessions.addAll(updatedSessions)
    readiness = readiness.copy(activeSessionId = null)
  }

  override fun sendMessage(sessionId: RuntimeSessionId, prompt: String): RuntimeTurnResult {
    val events =
      mutableListOf<RuntimeEvent>(
        RuntimeEvent.AssistantMessage(sessionId = sessionId, text = previewAssistantReply(prompt))
      )
    if (
      prompt.contains("approve", ignoreCase = true) ||
        prompt.contains("permission", ignoreCase = true)
    ) {
      events +=
        RuntimeEvent.ApprovalPending(
          RuntimeApprovalRequest(
            id = "approval-${prompt.hashCode().absoluteValue}",
            sessionId = sessionId,
            toolLabel = "shell",
            reason = "Approve this runtime action before continuing.",
          )
        )
    }
    return RuntimeTurnResult(sessionId = sessionId, events = events)
  }

  override fun submitApproval(
    requestId: String,
    decision: RuntimeApprovalDecision,
    sessionId: RuntimeSessionId,
  ): RuntimeTurnResult =
    RuntimeTurnResult(
      sessionId = sessionId,
      events =
        listOf(
          RuntimeEvent.AssistantMessage(
            sessionId = sessionId,
            text = "Approval $requestId recorded as ${decision.name.lowercase()}.",
          )
        ),
    )
}

private fun previewSessionId(index: Int): RuntimeSessionId =
  RuntimeSessionId("550e8400-e29b-41d4-a716-${index.toString().padStart(PREVIEW_SESSION_PAD, '0')}")

private fun previewAssistantReply(prompt: String): String =
  "Corvus runtime processed \"$prompt\" through the local mobile bridge."
