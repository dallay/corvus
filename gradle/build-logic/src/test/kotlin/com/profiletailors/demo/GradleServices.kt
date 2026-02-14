package com.profiletailors.demo

import com.profiletailors.plugin.serviceRegistry
import org.gradle.api.invocation.Gradle
import org.gradle.api.provider.Provider
import org.gradle.api.services.BuildService
import org.gradle.api.services.BuildServiceParameters
import org.gradle.execution.RunRootBuildWorkBuildOperationType
import org.gradle.internal.build.event.BuildEventListenerRegistryInternal
import org.gradle.internal.operations.BuildOperationDescriptor
import org.gradle.internal.operations.BuildOperationListener
import org.gradle.internal.operations.OperationFinishEvent
import org.gradle.internal.operations.OperationIdentifier
import org.gradle.internal.operations.OperationProgressEvent
import org.gradle.internal.operations.OperationStartEvent
import org.gradle.testkit.runner.TaskOutcome
import org.gradle.tooling.events.FinishEvent
import org.gradle.tooling.events.OperationCompletionListener
import org.gradle.tooling.events.task.TaskFailureResult
import org.gradle.tooling.events.task.TaskFinishEvent

/**
 * BuildDurationService
 *
 * A [build-service](https://docs.gradle.org/nightly/userguide/build_services.html) are
 * configuration
 * cacheable.[gradle-configuration-caching-replacing-a-build-listener-with-build-services](https://proandroiddev.com/gradle-configuration-caching-replacing-a-build-listener-with-build-services-40bde11937f1)
 */
abstract class BuildDurationService :
  BuildService<BuildServiceParameters.None>, BuildOperationListener {

  /** Build duration in milliseconds */
  var buildDuration: Long? = null

  /** Configuration duration in milliseconds */
  var configurationDuration: Long? = null

  /** Flag to indicate if configuration phase failed */
  var configurationPhaseFailed = true

  override fun started(p0: BuildOperationDescriptor, p1: OperationStartEvent) {
    // ...
  }

  override fun progress(p0: OperationIdentifier, p1: OperationProgressEvent) {
    // ...
  }

  override fun finished(
    buildOperationDescriptor: BuildOperationDescriptor,
    operationFinishEvent: OperationFinishEvent,
  ) {
    if (buildOperationDescriptor.details is RunRootBuildWorkBuildOperationType.Details) {
      /** Runs when build phase finishes, therefore we can assume configuration phase passed */
      configurationPhaseFailed = false

      val details = buildOperationDescriptor.details as RunRootBuildWorkBuildOperationType.Details?

      details?.buildStartTime?.let { buildStartTime ->
        buildDuration = System.currentTimeMillis() - buildStartTime

        val firstTaskStartTime = operationFinishEvent.startTime
        this.configurationDuration = firstTaskStartTime - buildStartTime
      }
    }
  }
}

/**
 * BuildTaskService
 *
 * [Task
 * outcomes](https://docs.gradle.org/nightly/userguide/more_about_tasks.html#sec:task_outcomes)
 */
abstract class BuildTaskService :
  BuildService<BuildServiceParameters.None>, OperationCompletionListener {

  var fromCacheTasksCount = 0
  var upToDateTasksCount = 0
  var skippedTasksCount = 0
  var noSourceTasksCount = 0
  var executedTasksCount = 0

  var buildPhaseFailureMessage: String? = null
  val buildPhaseFailed: Boolean
    get() = buildPhaseFailureMessage != null

  override fun onFinish(event: FinishEvent?) {
    if (event == null || event !is TaskFinishEvent) return

    when {
      event.isSkipped() -> {
        skippedTasksCount++
      }
      event.isNoSource() -> {
        noSourceTasksCount++
      }
      event.isFromCache() -> {
        fromCacheTasksCount++
      }
      event.isUpToDate() -> {
        upToDateTasksCount++
      }
      event.isSuccess() -> {
        executedTasksCount++
      }
    }

    if (event.result is TaskFailureResult) {
      buildPhaseFailureMessage =
        (event.result as TaskFailureResult).failures.firstOrNull()?.message
          ?: "${event.displayName} Failed without message"
    }
  }

  @Suppress("detekt:all")
  /*
   * The following functions are hacky workarounds to obtain a task execution result. They are what
   * I found to be the most consistent approach of obtaining this information I tried using similar
   * logic
   * [here](https://github.com/jrodbx/agp-sources/blob/3b6b17156dfcc8717c1bf217743cea8d15e034d2/7.1.3/com.android.tools.build/gradle/com/android/build/gradle/internal/profile/AnalyticsResourceManager.kt#L158).
   * but failed as explained [here](https://github.com/gradle/gradle/issues/5252). As of this date
   * and on gradle 7.4.2 they work. If task results are ever to change in the future, these will
   * need to be updated.
   */
  private fun FinishEvent.isFromCache(): Boolean {
    return this.displayName.endsWith(TaskOutcome.FROM_CACHE.name)
  }

  private fun FinishEvent.isUpToDate(): Boolean {
    return this.displayName.endsWith(TaskOutcome.UP_TO_DATE.name)
  }

  private fun FinishEvent.isSkipped(): Boolean {
    return this.displayName.endsWith(TaskOutcome.SKIPPED.name)
  }

  private fun FinishEvent.isNoSource(): Boolean {
    return this.displayName.endsWith(TaskOutcome.NO_SOURCE.name)
  }

  private fun FinishEvent.isSuccess(): Boolean {
    return this.displayName.endsWith(TaskOutcome.SUCCESS.name)
  }
}

@Suppress("unused")
object BuildServices {
  private fun registerBuildDurationService(gradle: Gradle): Provider<BuildDurationService> {
    val registry = gradle.serviceRegistry()[BuildEventListenerRegistryInternal::class.java]
    val buildDurationService =
      gradle.sharedServices.registerIfAbsent(
        "build-duration-service",
        BuildDurationService::class.java,
      ) {}

    registry.onOperationCompletion(buildDurationService)

    return buildDurationService
  }

  private fun registerBuildTaskService(gradle: Gradle): Provider<BuildTaskService> {
    val registry = gradle.serviceRegistry()[BuildEventListenerRegistryInternal::class.java]
    val buildTaskService =
      gradle.sharedServices.registerIfAbsent("build-task-service", BuildTaskService::class.java) {}

    registry.onTaskCompletion(buildTaskService)

    return buildTaskService
  }
}
