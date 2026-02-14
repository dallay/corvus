package com.profiletailors.demo

import com.profiletailors.plugin.utils.Ansi
import javax.inject.Inject
import org.gradle.api.DefaultTask
import org.gradle.api.Project
import org.gradle.api.artifacts.result.ResolvedDependencyResult
import org.gradle.api.file.ArchiveOperations
import org.gradle.api.file.ConfigurableFileCollection
import org.gradle.api.file.ConfigurableFileTree
import org.gradle.api.file.Directory
import org.gradle.api.file.DirectoryProperty
import org.gradle.api.file.FileCollection
import org.gradle.api.file.FileTree
import org.gradle.api.file.ProjectLayout
import org.gradle.api.file.RegularFile
import org.gradle.api.file.RegularFileProperty
import org.gradle.api.internal.file.FileOperations
import org.gradle.api.internal.tasks.JvmConstants
import org.gradle.api.model.ObjectFactory
import org.gradle.api.provider.ListProperty
import org.gradle.api.provider.MapProperty
import org.gradle.api.provider.Property
import org.gradle.api.provider.Provider
import org.gradle.api.provider.ProviderFactory
import org.gradle.api.provider.SetProperty
import org.gradle.api.tasks.IgnoreEmptyDirectories
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.InputDirectory
import org.gradle.api.tasks.InputFile
import org.gradle.api.tasks.InputFiles
import org.gradle.api.tasks.Internal
import org.gradle.api.tasks.Optional
import org.gradle.api.tasks.OutputDirectories
import org.gradle.api.tasks.OutputDirectory
import org.gradle.api.tasks.OutputFile
import org.gradle.api.tasks.OutputFiles
import org.gradle.api.tasks.PathSensitive
import org.gradle.api.tasks.PathSensitivity
import org.gradle.api.tasks.SkipWhenEmpty
import org.gradle.kotlin.dsl.property
import org.gradle.process.ExecOperations
import org.gradle.work.Incremental

// ===============================
// Gradle 属性类型统一别名
// 前缀：GradleProp
// https://docs.gradle.org/nightly/userguide/properties_providers.html#mutable_managed_properties
// ===============================

// ---------- 单值类型 ----------
/** 单个字符串属性 */
private typealias GradlePropStr = Property<String>

/** 单个整数属性 */
private typealias GradlePropInt = Property<Int>

/** 单个长整型属性 */
private typealias GradlePropLong = Property<Long>

/** 单个布尔属性 */
private typealias GradlePropBool = Property<Boolean>

/** 单个浮点数属性 */
private typealias GradlePropDouble = Property<Double>

// ---------- 文件/目录 ----------
/** 单个文件属性（推荐代替 Property<File>） */
private typealias GradlePropFile = RegularFileProperty

/** 单个目录属性（推荐代替 Property<File>） */
private typealias GradlePropDir = DirectoryProperty

// ---------- 集合类型 ----------
/** 列表属性 */
private typealias GradlePropList<T> = ListProperty<T>

/** 集合属性（去重，不保证顺序） */
private typealias GradlePropSet<T> = SetProperty<T>

/** 键值对属性 */
private typealias GradlePropMap<K, V> = MapProperty<K, V>

// ---------- 文件集合 ----------
/** 文件集合（多个离散文件/目录） */
private typealias GradlePropFiles = ConfigurableFileCollection

/** 文件树（包含层级结构，常用于源码树/资源树） */
private typealias GradlePropFileTree = ConfigurableFileTree

/**
 * annotating inputs and outputs
 *
 * [annotating_inputs_and_outputs](https://docs.gradle.org/nightly/userguide/implementing_custom_tasks.html#annotating_inputs_and_outputs)
 */
@Suppress("UnstableApiUsage")
private abstract class GradleAllTypes : DefaultTask() {

  // providers:
  // https://docs.gradle.org/nightly/userguide/lazy_configuration.html#lazy_configuration_reference

  abstract val readString: Provider<String>
  abstract val readFile: Provider<RegularFile>
  abstract val readDirectory: Provider<Directory>
  abstract val readFileCollection: Provider<FileCollection>
  abstract val readFileTree: Provider<FileTree>

  // inject:
  // https://docs.gradle.org/nightly/userguide/configuration_cache_requirements.html#config_cache:requirements:use_project_during_execution
  @get:Inject abstract val providers: ProviderFactory
  @get:Inject abstract val objects: ObjectFactory
  @get:Inject abstract val layout: ProjectLayout
  @get:Inject abstract val archives: ArchiveOperations
  @get:Inject abstract val files: FileOperations
  @get:Inject abstract val exec: ExecOperations

  // properties:
  // https://docs.gradle.org/nightly/userguide/lazy_configuration.html#property_files_api_reference

  // inputs
  @get:Input val inputString: Property<String> = objects.property<String>().convention("default")
  @get:InputFile
  val inputFile: RegularFileProperty =
    objects.fileProperty().convention(layout.buildDirectory.file("default.txt"))
  @get:InputDirectory
  val inputDirectory: DirectoryProperty =
    objects.directoryProperty().convention(layout.buildDirectory)
  @get:InputFiles
  val inputFileCollection: ConfigurableFileCollection =
    objects.fileCollection().convention(layout.buildDirectory.file("default.txt"))

  // outputs
  @get:OutputFile abstract val outputFile: RegularFileProperty
  @get:OutputDirectory abstract val outputDirectory: DirectoryProperty
  @get:OutputFiles abstract val outputFiles: ConfigurableFileCollection
  @get:OutputDirectories abstract val outputDirectories: ConfigurableFileCollection

  // other
  @get:Optional
  @get:InputFiles
  @get:Incremental
  @get:PathSensitive(PathSensitivity.RELATIVE)
  @get:SkipWhenEmpty
  @get:IgnoreEmptyDirectories
  @get:Internal
  abstract val pathSensitive: ConfigurableFileCollection

  /**
   * see https://docs.gradle.org/nightly/userguide/incremental_build.html#sec:runtime_api_for_adhoc
   */
  fun initInputsOutputs(
    inputString: String,
    inputFile: RegularFileProperty,
    inputDirectory: DirectoryProperty,
    inputFileTree: ConfigurableFileTree,
    outputFile: RegularFileProperty,
  ) {
    inputs.property("inputString", inputString).optional(true)
    inputs
      .file(inputFile)
      .withPropertyName("inputFile")
      .withPathSensitivity(PathSensitivity.RELATIVE)
      .skipWhenEmpty()
      .ignoreEmptyDirectories()
    inputs.dir(inputDirectory).withPropertyName("inputDirectory")
    inputs.files(inputFileTree).withPropertyName("inputFileTree")
    outputs.file(outputFile).withPropertyName("outputFile")
  }
}

@Suppress("detekt:all", "UnusedVariable")
private fun demos(project: Project) {
  //      layout.settingsDirectory
  //      layout.projectDirectory
  //      layout.buildDirectory
  //
  //      isolated.rootProject.projectDirectory
  //      isolated.projectDirectory
  //
  //      project.objects.domainObjectSet(File::class).add(file("build.gradle"))
  //      project.objects.namedDomainObjectSet(File::class).add(file("build.gradle"))
  //      project.objects.namedDomainObjectList(File::class).add(file("build.gradle"))
  //
  //      project.objects.domainObjectContainer(File::class).add(file("build.gradle"))
  //
  //      file("")
  //      files("")

  project.afterEvaluate {
    // demo1
    run {
      properties.forEach { (key, value) ->
        if (value is String) {
          println("${Ansi.color("$key=", Ansi.Color.GREEN.code)}$value")
        }
      }
    }

    // demo2
    // https://docs.gradle.org/nightly/userguide/how_to_upgrade_transitive_dependencies.html
    // https://docs.gradle.org/nightly/userguide/how_to_downgrade_transitive_dependencies.html
    // https://docs.gradle.org/nightly/userguide/how_to_exclude_transitive_dependencies.html
    // https://docs.gradle.org/nightly/userguide/how_to_prevent_accidental_dependency_upgrades.html
    run {
      configurations.configureEach {
        withDependencies {
          // 丢弃依赖
          removeIf { it.group == "commons-codec" && it.name == "commons-codec" }
        }
        // https://docs.gradle.org/nightly/userguide/resolution_rules.html
        resolutionStrategy {
          // 冲突则报错
          // 7. Force Failed Resolution Strategies
          // https://docs.gradle.org/nightly/userguide/resolution_rules.html#sec:conflict-resolution-strategy
          failOnVersionConflict()

          // 丢弃依赖
          // Capabilities
          // https://docs.gradle.org/nightly/userguide/component_capabilities.html
          // https://gradlex.org/jvm-dependency-conflict-resolution/#conflict
          capabilitiesResolution {
            withCapability("com.example:logging") { selectHighestVersion() }
          }

          // 固定版本
          // https://docs.gradle.org/nightly/userguide/how_to_downgrade_transitive_dependencies.html
          force("commons-codec:commons-codec:1.9")

          // 固定版本
          // 9. Dependency Resolve Rules and Other Conditionals
          // https://docs.gradle.org/nightly/userguide/resolution_rules.html#sec:dependency-resolve-rules
          eachDependency {
            val module = requested.module
            val group = requested.group
            val artifact = requested.name
            val version = requested.version
            if (group == "org.ow2.asm") {
              useVersion("9.8")
              because("Asm 9.8 is required for JDK 24 support")
            }
          }
        }
      }
    }

    // demo3
    run {
      val classPathDependencies =
        listOf(
          JvmConstants.COMPILE_CLASSPATH_CONFIGURATION_NAME,
          JvmConstants.TEST_COMPILE_CLASSPATH_CONFIGURATION_NAME,
          JvmConstants.RUNTIME_CLASSPATH_CONFIGURATION_NAME,
          JvmConstants.TEST_RUNTIME_CLASSPATH_CONFIGURATION_NAME,
        )
      val useFlat = true
      configurations
        .filter { config -> classPathDependencies.any { config.name == it } }
        .filter { it.isCanBeResolved }
        .forEach { config ->
          val dependencies =
            if (useFlat) config.incoming.resolutionResult.allDependencies
            else config.incoming.resolutionResult.root.dependencies

          dependencies.filterIsInstance<ResolvedDependencyResult>().forEach { dep ->
            val selected = dep.selected
            val moduleVersion = selected.moduleVersion ?: return@forEach
            val moduleId = "${moduleVersion.group}:${moduleVersion.name}"

            println("> Use dependency: $moduleId:${moduleVersion.version}")
          }
        }
    }

    // demo4
    run {
      val printActionableTasks = false
      if (printActionableTasks) {
        println()
        println("------------------------------------------------------------")
        println("List actionable tasks from project ':${project.name}'")
        println("------------------------------------------------------------")
        println()
        tasks
          .filter { !it.group.isNullOrBlank() && it.actions.isEmpty() }
          .groupBy { it.group ?: "other" }
          .forEach {
            println("\u001B[35m${it.key}\u001B[0m")
            it.value.forEach { taskInfo ->
              println(
                "  \u001B[32m${taskInfo.name}\u001B[0m\u001B[33m - ${taskInfo.description}\u001B[0m"
              )
            }
          }
      }
    }
  }
}
