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
 * 一个可缓存、可增量执行的文件内容检查任务。
 *
 * 用于扫描 src 下所有 .java / .kt 文件， 并根据 [contentCheckMap] 中配置的路径正则与内容正则， 检查文件内容中是否存在禁止的模式。
 *
 * 示例：
 *
 * ```kotlin
 * tasks.register<FileContentCheck>("fileContentChecker") {
 *   contentCheckMap.putAll(
 *     mapOf(".*\\/dallay/spring/." to listOf("import java\\.sql\\.(.*)"))
 *   )
 * }
 * ```
 *
 * 增量逻辑说明：
 * - 首次运行或输入不可增量时，会检查所有源文件；
 * - 之后仅检查修改/新增文件；
 * - 被删除的文件会被忽略。
 */
@Suppress("UnstableApiUsage")
@CacheableTask
abstract class FileContentCheck : DefaultTask(), Injected {

  /** 要扫描的源文件集合，支持增量、缓存、相对路径敏感。 */
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
   * 路径正则 -> 禁止内容正则列表。
   *
   * key 为文件路径正则（使用 invariantSeparatorsPath 匹配，路径分隔符统一为 "/"）； value 为对应文件中禁止出现的内容正则列表。
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
   * 检查单个文件是否违反内容规则。
   *
   * @param file 要检查的源文件
   * @param compiledRules 预编译好的路径与内容正则规则
   * @param violations 存储违规信息
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
