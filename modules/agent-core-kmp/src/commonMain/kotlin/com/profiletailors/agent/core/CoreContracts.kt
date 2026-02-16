package com.profiletailors.agent.core

data class CoreInvocation(
  val prompt: String,
  val sessionId: String? = null,
  val metadata: Map<String, String> = emptyMap(),
  val timeoutMs: Long? = null,
)

data class CoreOutput(val text: String, val transport: String, val rawOutput: String? = null)

sealed interface CoreResult {
  data class Success(val output: CoreOutput) : CoreResult

  data class Failure(
    val message: String,
    val details: String? = null,
    val recoverable: Boolean = false,
  ) : CoreResult
}

fun interface AgentCoreBridge {
  fun invoke(invocation: CoreInvocation): CoreResult
}
