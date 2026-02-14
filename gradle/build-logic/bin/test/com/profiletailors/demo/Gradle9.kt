@file:Suppress("unused", "UnusedReceiverParameter")

package com.profiletailors.demo

import org.gradle.api.DefaultTask
import org.gradle.api.Project
import org.gradle.api.reporting.dependencies.HtmlDependencyReportTask
import org.gradle.api.tasks.SourceSet
import org.gradle.api.tasks.SourceSetContainer
import org.gradle.api.tasks.bundling.AbstractArchiveTask
import org.gradle.api.tasks.bundling.War
import org.gradle.api.tasks.diagnostics.DependencyReportTask
import org.gradle.api.tasks.diagnostics.PropertyReportTask
import org.gradle.api.tasks.diagnostics.TaskReportTask
import org.gradle.kotlin.dsl.named
import org.gradle.kotlin.dsl.withType

internal object Gradle9 {
  // https://docs.gradle.org/current/userguide/upgrading_major_version_9.html

  /**
   * 应尽量使用惰性APIs
   * - [https://docs.gradle.org/nightly/userguide/lazy_eager_evaluation.html](https://docs.gradle.org/nightly/userguide/lazy_eager_evaluation.html)
   * - [https://docs.gradle.org/current/userguide/task_configuration_avoidance.html#sec:old_vs_new_configuration_api_overview](https://docs.gradle.org/current/userguide/task_configuration_avoidance.html#sec:old_vs_new_configuration_api_overview)
   * - [https://github.com/gradle/gradle/issues/29384](https://github.com/gradle/gradle/issues/29384)
   */
  private fun Project.lazyApisToUse(): List<String> {
    return listOf(
        "org.gradle.api.tasks.TaskContainer.register()",
        "org.gradle.api.tasks.TaskCollection.named()",
        "org.gradle.api.tasks.TaskCollection.withType()",
        "org.gradle.api.tasks.TaskCollection.configureEach()",
        "org.gradle.api.plugins.PluginManager.withPlugin()",
      )
      .ifEmpty {
        tasks.register("api1") { println("api1") }
        tasks.named<DefaultTask>("api2")
        tasks.withType<War>().configureEach { webXml = file("src/someWeb.xml") }
        tasks.configureEach { println("api4") }
        pluginManager.withPlugin("java") {
          extensions
            .getByType(SourceSetContainer::class.java)
            .getByName(SourceSet.MAIN_SOURCE_SET_NAME)
            .java
            .srcDirs(listOf("src"))
        }
        listOf()
      }
  }

  /**
   * In Gradle 9.0, the default behavior of archive tasks (such as Jar, Ear, War, Zip, and
   * AbstractArchiveTask) has changed to produce reproducible archives by default.
   * https://docs.gradle.org/current/userguide/upgrading_major_version_9.html#reproducible_archives_by_default
   */
  private fun Project.archiveTaskBehaviorChanged() {
    tasks.withType<AbstractArchiveTask>().configureEach {
      isPreserveFileTimestamps = false
      isReproducibleFileOrder = true
      filePermissions { unix("0664") }
      dirPermissions { unix("0775") }
    }
  }

  /**
   * Gradle no longer implicitly builds certain artifacts during assemble
   * https://docs.gradle.org/current/userguide/upgrading_major_version_9.html#gradle_no_longer_implicitly_builds_certain_artifacts_during_assemble
   */
  private fun Project.artifactsVisibleFalseNoLongerInAssemble() {
    tasks.named("assemble").configure { dependsOn(tasks.withType<AbstractArchiveTask>()) }
  }

  /**
   * https://docs.gradle.org/current/userguide/upgrading_major_version_9.html#removal_of_conventions
   */
  private fun Project.conventionsRemoved() {
    println(
      mapOf(
        "project.projectReports" to
          listOf(
            TaskReportTask::class,
            PropertyReportTask::class,
            DependencyReportTask::class,
            HtmlDependencyReportTask::class,
          )
      )
    )
  }

  /**
   * https://docs.gradle.org/current/userguide/upgrading_major_version_9.html#removal_of_name_task_reference_syntax_in_kotlin_dsl
   */
  private fun Project.usingNamedInKotlinDsl() {
    println("using named<XXX>() in kotlin dsl")
  }
}
