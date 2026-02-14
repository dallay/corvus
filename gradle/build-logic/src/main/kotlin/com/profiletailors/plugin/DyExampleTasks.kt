package com.profiletailors.plugin

import com.profiletailors.plugin.utils.Ansi
import org.gradle.api.DefaultTask
import org.gradle.api.file.ConfigurableFileCollection
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.file.FileType
import org.gradle.api.tasks.IgnoreEmptyDirectories
import org.gradle.api.tasks.InputFiles
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.PathSensitive
import org.gradle.api.tasks.PathSensitivity
import org.gradle.api.tasks.TaskAction
import org.gradle.api.tasks.UntrackedTask
import org.gradle.work.ChangeType
import org.gradle.work.DisableCachingByDefault
import org.gradle.work.Incremental
import org.gradle.work.InputChanges

/**
 * dy copy example task
 *
 * only copy files that have changed to the output directory
 *
 * [org.gradle.api.Project.files] itself does not actively traverse subdirectories, but some Gradle
 * APIs, such as [InputChanges.getFileChanges] will automatically recurse into files in the
 * directory when consuming it.
 *
 * See
 * [file_trees](https://docs.gradle.org/nightly/userguide/working_with_files.html#sec:file_trees)
 *
 * usage:
 * ```
 * tasks.register<DyCopyExampleTask>("dyCopyExample") {
 *   group = "other"
 *   description = "dy copy example task"
 *   from = files("src/main/resources")
 *   to = layout.buildDirectory.dir("dy/resources")
 * }
 * ```
 *
 * links:
 * - [task-development](https://docs.gradle.org/nightly/userguide/controlling_task_execution.html#task-development)
 * - [creating_lazy_inputs_and_outputs](https://docs.gradle.org/nightly/userguide/implementing_custom_tasks.html#creating_lazy_inputs_and_outputs)
 * - [cacheable
 *   tasks](https://docs.gradle.org/nightly/userguide/build_cache.html#sec:task_output_caching_details)
 * - [incremental_build](https://docs.gradle.org/nightly/userguide/incremental_build.html)
 * - [managed_properties](https://docs.gradle.org/nightly/userguide/properties_providers.html#managed_properties)
 * - [available_collections](https://docs.gradle.org/nightly/userguide/collections.html#available_collections)
 * - [working_with_files](https://docs.gradle.org/nightly/userguide/working_with_files.html)
 *
 * see [org.gradle.api.file.FileSystemOperations]
 */
@DisableCachingByDefault(because = "this is a demo task")
@UntrackedTask(because = "this is a demo task")
abstract class DyCopyExampleTask : DefaultTask(), Injected {

  @get:InputFiles
  @get:Incremental
  @get:PathSensitive(PathSensitivity.RELATIVE)
  @get:IgnoreEmptyDirectories
  abstract val from: ConfigurableFileCollection

  @get:OutputDirectory abstract val to: DirectoryProperty

  init {
    group = "other"
    description = "dy copy example task"
  }

  @TaskAction
  fun execute(inputChanges: InputChanges) {
    val targetDir = to.asFile.get()
    val buildDir = layout.buildDirectory.asFile.get()
    if (
      !targetDir.invariantSeparatorsPath.startsWith(buildDir.invariantSeparatorsPath) ||
        targetDir == buildDir
    ) {
      error("Output directory must be a subdirectory of the build directory.")
    }

    println(if (inputChanges.isIncremental) "incremental build:" else "full build:")
    inputChanges.getFileChanges(from).forEach { change ->
      if (change.fileType != FileType.FILE) return@forEach
      val changeFile = change.file
      val destFile = targetDir.resolve(changeFile.name)

      when (change.changeType) {
        ChangeType.ADDED,
        ChangeType.MODIFIED -> {
          changeFile.copyTo(destFile, true)
          println(
            Ansi.color(
              """
                      |  sync file $changeFile
                      |    to $destFile
                      """
                .trimMargin(),
              if (change.changeType == ChangeType.ADDED) Ansi.Color.GREEN.code
              else Ansi.Color.YELLOW.code,
            )
          )
        }
        ChangeType.REMOVED -> {
          if (destFile.exists()) {
            destFile.delete()
            println(
              Ansi.color(
                """
                        |  delete file $changeFile
                        |    and $destFile
                        """
                  .trimMargin(),
                Ansi.Color.RED.code,
              )
            )
          }
        }
      }
    }
  }
}
