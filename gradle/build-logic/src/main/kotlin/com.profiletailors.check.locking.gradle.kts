@file:Suppress("UnstableApiUsage")

import java.io.File
import java.nio.charset.StandardCharsets
import java.util.concurrent.TimeUnit

val isCi = providers.environmentVariable("CI").orNull?.isNotBlank() == true

val excludedLockingConfigurationPrefixes = listOf("spotless", "detachedConfiguration")

val excludedLockingConfigurations =
  setOf(
    "combinedGraphClasspath",
    "projectHealthClasspath",
    "projectMetadataClasspath",
    "resolvedDepsClasspath",
  )

val buildLogicOnlyExcludedLockingConfigurations =
  setOf(
    "precompiledScriptPluginAccessorsGenerationClasspath",
    "runtimeClasspath",
    "testRuntimeClasspath",
  )

fun Configuration.shouldUseDependencyLocking(): Boolean {
  if (!isCanBeResolved) {
    return false
  }

  if (name in excludedLockingConfigurations) {
    return false
  }

  if (
    project.rootProject.name == "corvus-build-logic" &&
      name in buildLogicOnlyExcludedLockingConfigurations
  ) {
    return false
  }

  return excludedLockingConfigurationPrefixes.none { prefix -> name.startsWith(prefix) }
}

fun findGradleWrapper(startDir: File): File? {
  val wrapperName =
    if (org.gradle.internal.os.OperatingSystem.current().isWindows) "gradlew.bat" else "gradlew"

  return generateSequence(startDir) { it.parentFile }
    .map { it.resolve(wrapperName) }
    .firstOrNull { it.isFile }
}

dependencyLocking { ignoredDependencies.add("com.example:*") }

buildscript.configurations.configureEach {
  if (shouldUseDependencyLocking()) {
    resolutionStrategy {
      cacheDynamicVersionsFor(7, TimeUnit.DAYS)
      activateDependencyLocking()
    }
  }
}

configurations.configureEach {
  if (shouldUseDependencyLocking()) {
    resolutionStrategy {
      cacheDynamicVersionsFor(7, TimeUnit.DAYS)
      activateDependencyLocking()
    }
  }
}

val lockFilesProvider = provider {
  listOf(
    layout.projectDirectory.file("buildscript-gradle.lockfile").asFile,
    layout.projectDirectory.file("gradle.lockfile").asFile,
  )
}

val gradleWrapperProvider = provider { findGradleWrapper(rootDir) }
val dependenciesTaskPath = provider { if (path == ":") "dependencies" else "$path:dependencies" }

val writeLocks =
  tasks.register<Exec>("writeLocks") {
    group = "toolbox"
    description = "Write dependency lockfiles for ${project.path}."
    notCompatibleWithConfigurationCache("Runs nested Gradle commands to refresh dependency locks.")

    val wrapper =
      gradleWrapperProvider.get()
        ?: error("Could not locate Gradle wrapper starting from ${rootDir.absolutePath}")

    workingDir = rootDir
    commandLine(wrapper.absolutePath, dependenciesTaskPath.get(), "--write-locks")

    doFirst {
      lockFilesProvider.get().forEach { file ->
        if (file.exists()) {
          val backup = layout.buildDirectory.file("tmp/locks/${file.name}.bak").get().asFile
          backup.parentFile.mkdirs()
          file.copyTo(backup, overwrite = true)
        }
      }
    }

    doLast {
      if (!org.gradle.internal.os.OperatingSystem.current().isUnix) {
        lockFilesProvider.get().forEach { file ->
          if (file.exists()) {
            file.writeText(
              file.readText().replace(System.lineSeparator(), "\n"),
              StandardCharsets.UTF_8,
            )
          }
        }
      }
    }
  }

tasks.register("checkLocks") {
  group = "toolbox"
  description = "Verify dependency lockfiles for ${project.path}."
  notCompatibleWithConfigurationCache("Runs nested Gradle commands to validate dependency locks.")
  dependsOn(writeLocks)

  doLast {
    lockFilesProvider.get().forEach { file ->
      val backup = layout.buildDirectory.file("tmp/locks/${file.name}.bak").get().asFile
      if (backup.exists() && file.exists()) {
        val backupContent = backup.readText()
        val currentContent = file.readText()
        if (backupContent != currentContent) {
          throw GradleException(
            "${file.absolutePath} changed, please run './gradlew writeLocksAll' and commit the updates"
          )
        }
      }
    }
  }
}
