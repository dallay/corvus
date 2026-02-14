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
      val resolved =
        aggregatedClasspath.get().find {
          val resolvedId = it.id
          resolvedId is ModuleComponentIdentifier && resolvedId.moduleIdentifier.toString() == id
        }
      if (resolved == null) {
        if (!unUsedExcludes.get().contains(id)) {
          "Not used: $id:$version\n"
            .also {
              if (failOnUnUsed.orNull == true) {
                errors += it
              }
              issues += it
            }
        }
      } else {
        val resolvedVersion = resolved.moduleVersion?.version
        if (resolvedVersion != version) {
          "Wrong version: $id (declared=$version; used=$resolvedVersion)\n"
            .also {
              errors += it
              issues += it
            }
        }
      }
    }

    reportFile.get().asFile.writeText(issues)

    if (!errors.isEmpty()) {
      error(errors)
    }
  }
}
