package com.profiletailors.plugin.tasks

import com.profiletailors.plugin.Injected
import java.io.File
import java.nio.charset.StandardCharsets
import org.gradle.api.DefaultTask
import org.gradle.api.GradleException
import org.gradle.api.file.ConfigurableFileCollection
import org.gradle.api.file.FileType
import org.gradle.api.provider.MapProperty
import org.gradle.api.tasks.CacheableTask
import org.gradle.api.tasks.IgnoreEmptyDirectories
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputFiles
import org.gradle.api.tasks.PathSensitive
import org.gradle.api.tasks.PathSensitivity
import org.gradle.api.tasks.TaskAction
import org.gradle.work.ChangeType
import org.gradle.work.Incremental
import org.gradle.work.InputChanges

/**
 * FileContentCheck
 *
 * A cacheable, incrementally executable file content check task.
 *
 * Scans all .java / .kt files under src, and checks whether forbidden patterns exist in the file
 * content based on the path regex and content regex configured in [contentCheckMap].
 *
 * Example:
 * ```kotlin
 * tasks.register<FileContentCheck>("fileContentChecker") {
 *   contentCheckMap.putAll(
 *     mapOf(".*\\/dallay/spring/." to listOf("import java\\.sql\\.(.*)"))
 *   )
 * }
 * ```
 *
 * Incremental logic description:
 * - Upon first run or when inputs are not incremental, all source files are checked;
 * - Thereafter, only modified/new files are checked;
 * - Deleted files are ignored.
 */
@Suppress("UnstableApiUsage")
@CacheableTask
abstract class FileContentCheck : DefaultTask(), Injected {

  /**
   * Collection of source files to scan, supporting incremental, caching, and relative path
   * sensitivity.
   */
  @get:InputFiles
  @get:Incremental
  @get:PathSensitive(PathSensitivity.RELATIVE)
  @get:IgnoreEmptyDirectories
  val sourceFiles: ConfigurableFileCollection =
    objects
      .fileCollection()
      .convention(
        layout.projectDirectory.dir("src").asFileTree.matching { include("**/*.java", "**/*.kt") }
      )

  /**
   * Path regex -> Forbidden content regex list.
   *
   * key is the file path regex (matched using invariantSeparatorsPath, path separator unified as
   * "/"); value is the list of forbidden content regexes in the corresponding file.
   */
  @get:Input abstract val contentCheckMap: MapProperty<String, List<String>>

  init {
    description = "Check file contents with incremental build and regex path matching"
    outputs.upToDateWhen { true }
  }

  @TaskAction
  fun execute(inputs: InputChanges) {
    if (!contentCheckMap.isPresent || contentCheckMap.get().isEmpty()) return

    val violations = mutableListOf<String>()

    val compiled: List<Pair<Regex, List<Regex>>> by lazy {
      contentCheckMap.get().map { (pathPattern, contentRegexList) ->
        val pathRegex = Regex(pathPattern)
        val compiledContentRegexes = contentRegexList.map { Regex(it) }
        pathRegex to compiledContentRegexes
      }
    }

    if (!inputs.isIncremental) {
      sourceFiles.forEach { checkFile(it, compiled, violations) }
    } else {
      inputs.getFileChanges(sourceFiles).forEach { change ->
        if (change.fileType != FileType.FILE) return@forEach
        when (change.changeType) {
          ChangeType.REMOVED -> {}
          else -> checkFile(change.file, compiled, violations)
        }
      }
    }

    if (violations.isNotEmpty()) {
      throw GradleException(violations.joinToString("\n"))
    }
  }

  /**
   * Check if a single file violates content rules.
   *
   * @param file The source file to check
   * @param compiledRules Pre-compiled path and content regex rules
   * @param violations Stores violation information
   */
  @Suppress("detekt:NestedBlockDepth")
  private fun checkFile(
    file: File,
    compiledRules: List<Pair<Regex, List<Regex>>>,
    violations: MutableList<String>,
  ) {
    val path = file.invariantSeparatorsPath
    val text = file.readText(StandardCharsets.UTF_8)

    compiledRules.forEach { (pathRegex, contentRegexes) ->
      if (pathRegex.containsMatchIn(path)) {
        contentRegexes.forEach { regex ->
          if (regex.containsMatchIn(text)) {
            violations +=
              "${file.path} violates content rule: ${regex.pattern} (matched by path pattern: ${pathRegex.pattern})"
          }
        }
      }
    }
  }
}
