package com.profiletailors.fixtures

import java.io.File
import java.lang.management.ManagementFactory
import java.nio.file.Files
import org.gradle.testkit.runner.BuildResult
import org.gradle.testkit.runner.GradleRunner

/**
 * Access to a minimal project inside a temporary folder. The project contain files that are
 * expected to exist in our setup.
 */
class GradleProject {

  val projectDir: File = Files.createTempDirectory("test-project").toFile()
  val gradlePropertiesFile = file("gradle.properties")
  val settingsFile = file("settings.gradle.kts")
  val rootBuildFile = file("build.gradle.kts")
  val catalog = file("gradle/libs.versions.toml")
  val versions = file("gradle/versions/build.gradle.kts")
  val aggregation = file("gradle/aggregation/build.gradle.kts")
  val moduleBuildFile = file("module/build.gradle.kts")
  val moduleBuildPropertiesFile = file("module/build.properties")

  fun settingsFile(content: String) = settingsFile.also { it.writeText(content) }

  fun rootBuildFile(content: String) = rootBuildFile.also { it.writeText(content) }

  fun catalog(content: String) = catalog.also { it.writeText(content) }

  fun moduleBuildFile(content: String) = moduleBuildFile.also { it.writeText(content) }

  fun moduleBuildPropertiesFile(content: String) =
    moduleBuildPropertiesFile.also { it.writeText(content) }

  fun file(path: String, content: String? = null) =
    File(projectDir, path).also {
      it.parentFile.mkdirs()
      if (content != null) {
        it.writeText(content)
      }
    }

  fun help(): BuildResult = runner(listOf("help")).build()

  fun build(): BuildResult = runner(listOf("build")).build()

  fun runner(args: List<String>): GradleRunner =
    GradleRunner.create()
      .forwardOutput()
      .withGradleVersion("9.2.0")
      .withPluginClasspath()
      .withProjectDir(projectDir)
      .withArguments(args + listOf("-s", "--warning-mode=all"))
      .withDebug(
        ManagementFactory.getRuntimeMXBean().inputArguments.toString().contains("-agentlib:jdwp")
      )
}

internal fun consoleLog(message: String, enable: Boolean = false) {
  if (enable) println(message)
}
