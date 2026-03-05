package com.profiletailors.plugin.tasks

import org.gradle.api.DefaultTask
import org.gradle.api.artifacts.component.ModuleComponentIdentifier
import org.gradle.api.artifacts.result.ResolvedComponentResult
import org.gradle.api.file.RegularFileProperty
import org.gradle.api.provider.ListProperty
import org.gradle.api.provider.MapProperty
import org.gradle.api.provider.Property
import org.gradle.api.provider.SetProperty
import org.gradle.api.tasks.CacheableTask
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.Optional
import org.gradle.api.tasks.OutputFile
import org.gradle.api.tasks.TaskAction

/** Check that all versions declared in a java-platform build.gradle.kts file are actually used. */
@CacheableTask
abstract class JavaVersionConsistencyCheck : DefaultTask() {

  private data class ComparisonIssue(val message: String, val isError: Boolean)

  /** The versions declared in the build.gradle.kts file. */
  @get:Input abstract val definedVersions: MapProperty<String, String>

  /** The aggregated classpath of all modules using the versions to resolve their dependencies. */
  @get:Input abstract val aggregatedClasspath: SetProperty<ResolvedComponentResult>

  /** Whether to fail if there are unused versions. */
  @get:Input @get:Optional abstract val failOnUnUsed: Property<Boolean>

  /**
   * List of versions to ignore. This may be needed if versions for components that are not part of
   * the runtime module path of the applications are managed.
   */
  @get:Input abstract val unUsedExcludes: ListProperty<String>

  /** The report TXT file that will contain the issues found. */
  @get:OutputFile abstract val reportFile: RegularFileProperty

  @TaskAction
  fun compare() {
    var errors = ""
    var issues = ""

    definedVersions.get().forEach { (id, version) ->
      val resolved = findResolvedComponent(id)
      val issue = compareVersion(id, version, resolved)
      if (issue != null) {
        issues += issue.message
        if (issue.isError) {
          errors += issue.message
        }
      }
    }

    reportFile.get().asFile.writeText(issues)

    if (!errors.isEmpty()) {
      error(errors)
    }
  }

  private fun findResolvedComponent(id: String): ResolvedComponentResult? {
    return aggregatedClasspath.get().find {
      val resolvedId = it.id
      resolvedId is ModuleComponentIdentifier && resolvedId.moduleIdentifier.toString() == id
    }
  }

  private fun compareVersion(
    id: String,
    expectedVersion: String,
    resolved: ResolvedComponentResult?,
  ): ComparisonIssue? {
    if (resolved == null) {
      return notUsedIssue(id, expectedVersion)
    }

    val resolvedVersion = resolved.moduleVersion?.version
    if (resolvedVersion == expectedVersion) {
      return null
    }

    return ComparisonIssue(
      message = "Wrong version: $id (declared=$expectedVersion; used=$resolvedVersion)\n",
      isError = true,
    )
  }

  private fun notUsedIssue(id: String, version: String): ComparisonIssue? {
    if (unUsedExcludes.get().contains(id)) {
      return null
    }
    return ComparisonIssue(
      message = "Not used: $id:$version\n",
      isError = failOnUnUsed.orNull == true,
    )
  }
}
