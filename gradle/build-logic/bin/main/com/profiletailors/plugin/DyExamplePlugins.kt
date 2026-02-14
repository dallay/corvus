@file:Suppress("UnstableApiUsage")

package com.profiletailors.plugin

import com.profiletailors.plugin.utils.Ansi
import com.profiletailors.plugin.utils.CatalogUtil
import org.gradle.api.Action
import org.gradle.api.DefaultTask
import org.gradle.api.Plugin
import org.gradle.api.Project
import org.gradle.api.initialization.Settings
import org.gradle.api.invocation.Gradle
import org.gradle.api.plugins.PluginAware
import org.gradle.api.provider.ListProperty
import org.gradle.api.provider.Property
import org.gradle.api.tasks.Input
import org.gradle.api.tasks.Nested
import org.gradle.api.tasks.TaskAction
import org.gradle.internal.Actions.with
import org.gradle.kotlin.dsl.embeddedKotlinVersion
import org.gradle.work.DisableCachingByDefault

/*
   初始化阶段 (Initialization)
    ├── beforeSettings {}
    ├── 解析 settings.gradle
    ├── settingsEvaluated {}
    ├── projectsLoaded {}

   配置阶段 (Configuration)
    ├── beforeProject { Project }
    ├── 执行 build.gradle
    ├── afterProject { Project }
    ├── 所有项目完成 -> projectsEvaluated {}

   执行阶段 (Execution)
    ├── 构建 TaskExecutionGraph
    ├── taskGraph.whenReady {}
    ├── 执行 Task:
    │     ├── doFirst {}
    │     ├── action
    │     └── doLast {}
    └── buildFinished { BuildResult }
*/

/** id("com.profiletailors.plugin.dy.example.settings") */
class DyExampleSettingsPlugin : Plugin<Settings> {
  override fun apply(target: Settings) {
    with(target) {
      println(
        Ansi.color("> Gradle Version (${settings.gradle.gradleVersion})", Ansi.Color.GREEN.code) +
          Ansi.color(" Kotlin Version (${embeddedKotlinVersion})", Ansi.Color.GREEN.code)
      )
      settings.gradle.beforeProject {
        val projectDir = isolated.projectDirectory.asFile.invariantSeparatorsPath
        // see LifecycleProjectEvaluator
        logger.lifecycle("> Configure project :$projectDir")
      }
      val printPlugin = false
      if (printPlugin) {
        settings.pluginManagement.resolutionStrategy {
          eachPlugin {
            val id = requested.id.id
            val version = requested.version
            println(
              "> Apply plugin: ${id}:${version}"
                .let {
                  run {
                    val pageUrl = CatalogUtil.getPluginPageUrl(id)
                    val metadataUrl = CatalogUtil.getPluginMetadataUrl(id)
                    "$it\n  ${pageUrl}\n  $metadataUrl"
                  }
                }
            )
          }
        }
      }
    }
  }
}

/**
 * id("com.profiletailors.plugin.dy.example.project")
 * - [developing_binary_plugin_advanced](https://docs.gradle.org/nightly/userguide/developing_binary_plugin_advanced.html)
 * - [plugin-development](https://docs.gradle.org/nightly/userguide/implementing_gradle_plugins_binary.html#plugin-development)
 * - [task_input_output_annotations](https://docs.gradle.org/nightly/userguide/incremental_build.html#sec:task_input_output_annotations)
 */
class DyExampleProjectPlugin : Plugin<Project> {

  // -------- DSL 对象定义 --------

  /** prop 配置，用于控制是否打印项目属性 */
  interface PropConfig {
    @get:Input val enabled: Property<Boolean>
  }

  /** dep 配置，用于控制是否打印依赖 */
  interface DepConfig {
    @get:Input val enabled: Property<Boolean>

    @get:Input val flat: Property<Boolean>

    @get:Nested val rules: DepRuleConfig

    fun rules(action: Action<DepRuleConfig>) = action.execute(rules)
  }

  /** 二层嵌套规则配置（可选） */
  interface DepRuleConfig {
    @get:Input val includes: ListProperty<String>

    @get:Input val excludes: ListProperty<String>

    fun include(vararg patterns: String) = includes.addAll(patterns.asList())

    fun exclude(vararg patterns: String) = excludes.addAll(patterns.asList())
  }

  // -------- 插件扩展 (顶级 DSL) --------

  interface MyPluginExtension {
    @get:Nested val propConfig: PropConfig
    @get:Nested val depConfig: DepConfig

    fun prop(action: Action<PropConfig>) = action.execute(propConfig)

    fun dep(action: Action<DepConfig>) = action.execute(depConfig)
  }

  // -------- Task 定义 --------
  @DisableCachingByDefault(because = "Not cacheable")
  abstract class MyTask : DefaultTask(), Injected {

    @get:Input abstract val projectName: Property<String>

    @get:Nested abstract val propConfig: PropConfig
    @get:Nested abstract val depConfig: DepConfig

    fun config(prop: PropConfig, dep: DepConfig) {
      this.projectName.set(providers.provider { project.name })
      this.propConfig.enabled.set(prop.enabled.convention(true))
      this.depConfig.enabled.set(dep.enabled.convention(true))
      this.depConfig.flat.set(dep.flat.convention(true))
      this.depConfig.rules.includes.set(dep.rules.includes.convention(listOf("aaa")))
      this.depConfig.rules.excludes.set(dep.rules.excludes.convention(listOf("bbb")))
    }

    @TaskAction
    fun run() {
      println("=== MyTask Run In Project: ${projectName.get()} ===")
      val gradleIssue =
        """
        |🤡 Oh Gradle:
        |  https://github.com/gradle/gradle/issues/31132
        |  https://docs.gradle.org/nightly/userguide/configuration_cache_status.html
        |  https://docs.gradle.org/nightly/userguide/configuration_cache_requirements.html#config_cache:requirements:use_project_during_execution
        |  https://github.com/gradle/gradle/blob/master/platforms/core-configuration/core-serialization-codecs/src/main/kotlin/org/gradle/internal/serialize/codecs/core/UnsupportedTypesCodecs.kt
        |"""
          .trimMargin()

      // 打印项目属性
      if (propConfig.enabled.get()) {
        println("=== Project Properties ===")
        println("  $gradleIssue")
      }

      // 打印依赖
      if (depConfig.enabled.get()) {
        val rules = depConfig.rules
        println("\n=== Dependencies Used ===")
        println("  include: ${rules.includes.get()}")
        println("  exclude: ${rules.excludes.get()}")
        println("  $gradleIssue")
      }
    }
  }

  // -------- 插件入口 --------

  override fun apply(target: Project) {
    with(target) {
      val ext = extensions.create("dyPlugin", MyPluginExtension::class.java)
      tasks.register("dyExample", MyTask::class.java) {
        // notCompatibleWithConfigurationCache()
        config(ext.propConfig, ext.depConfig)
      }
    }
  }
}

/**
 * id("com.profiletailors.plugin.dy.example.aware")
 *
 * plugin type: `InitPlugin` `SettingsPlugin` `ProjectPlugin`
 *
 * [plugin_scope](https://docs.gradle.org/nightly/userguide/plugin_introduction_advanced.html#plugin_scope)
 */
class DyExampleAwarePlugin : Plugin<PluginAware> {

  override fun apply(target: PluginAware) {
    when (target) {
      is Project -> {
        println(Ansi.color("you are using a project plugin", Ansi.Color.BLUE.code))
      }
      is Settings -> {
        println(Ansi.color("you are using a settings plugin", Ansi.Color.BLUE.code))
      }
      is Gradle -> {
        println(Ansi.color("you are using a init plugin", Ansi.Color.BLUE.code))
      }
      else -> error("Unknown PluginAware type ${target.javaClass.name}")
    }
  }
}
