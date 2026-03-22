@file:Suppress("UnstableApiUsage")

import java.io.File
import java.nio.charset.StandardCharsets
import java.util.concurrent.TimeUnit

val isCi = providers.environmentVariable("CI").orNull?.isNotBlank() == true
val safeNettyVersion = "4.1.118.Final"
val safeProtobufVersion = "3.25.5"
val safeJacksonToolsVersion = "3.1.0"

val excludedLockingConfigurationPrefixes =
  listOf("allDevSourceSets", "composeHotReloadDev", "detachedConfiguration", "jvmDev", "spotless")

val excludedLockingConfigurations =
  setOf(
    "combinedGraphClasspath",
    "commonTestResolvableDependenciesMetadata",
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

val composeAppOsSpecificExcludedLockingConfigurations =
  setOf(
    "allSourceSetsCompileDependenciesMetadata",
    "allTestSourceSetsCompileDependenciesMetadata",
    "jvmCompileClasspath",
    "jvmRuntimeClasspath",
    "jvmMainCompileClasspath",
    "jvmMainResolvableDependenciesMetadata",
    "jvmMainRuntimeClasspath",
    "jvmTestCompileClasspath",
    "jvmTestResolvableDependenciesMetadata",
    "jvmTestRuntimeClasspath",
  )

fun Configuration.shouldUseDependencyLocking(): Boolean {
  val isBuildLogicOnlyExcluded =
    project.rootProject.name == "corvus-build-logic" &&
      name in buildLogicOnlyExcludedLockingConfigurations
  val isComposeAppOsSpecificExcluded =
    project.path == ":composeApp" && name in composeAppOsSpecificExcludedLockingConfigurations
  val hasExcludedPrefix =
    excludedLockingConfigurationPrefixes.any { prefix -> name.startsWith(prefix) }

  return isCanBeResolved &&
    name !in excludedLockingConfigurations &&
    !isBuildLogicOnlyExcluded &&
    !isComposeAppOsSpecificExcluded &&
    !hasExcludedPrefix
}

fun ResolutionStrategy.enforceSafeNettyVersion() {
  eachDependency {
    if (requested.group == "io.netty" && requested.version != safeNettyVersion) {
      useVersion(safeNettyVersion)
      because("Netty $safeNettyVersion fixes the native SslHandler crash vulnerability")
    }
  }
}

fun ResolutionStrategy.enforceSafeProtobufVersion() {
  eachDependency {
    if (requested.group == "com.google.protobuf" && requested.version != safeProtobufVersion) {
      useVersion(safeProtobufVersion)
      because(
        "Protobuf $safeProtobufVersion mitigates the parser recursion denial of service issue"
      )
    }
  }
}

fun ResolutionStrategy.enforceSafeJacksonToolsVersion() {
  eachDependency {
    if (
      requested.group?.startsWith("tools.jackson") == true &&
        requested.version != safeJacksonToolsVersion
    ) {
      useVersion(safeJacksonToolsVersion)
      because(
        "Jackson Tools $safeJacksonToolsVersion fixes the nesting depth constraint bypass in jackson-core"
      )
    }
  }
}

fun findGradleWrapper(startDir: File): File? {
  val wrapperName =
    if (org.gradle.internal.os.OperatingSystem.current().isWindows) "gradlew.bat" else "gradlew"

  return generateSequence(startDir) { it.parentFile }
    .map { it.resolve(wrapperName) }
    .firstOrNull { it.isFile }
}

dependencyLocking {
  ignoredDependencies.add("com.example:*")
  if (isCi) {
    lockMode = LockMode.STRICT
  }
}

buildscript.configurations.configureEach {
  if (shouldUseDependencyLocking()) {
    resolutionStrategy {
      cacheDynamicVersionsFor(7, TimeUnit.DAYS)
      enforceSafeNettyVersion()
      enforceSafeProtobufVersion()
      enforceSafeJacksonToolsVersion()
      activateDependencyLocking()
    }
  }
}

configurations.configureEach {
  if (shouldUseDependencyLocking()) {
    resolutionStrategy {
      cacheDynamicVersionsFor(7, TimeUnit.DAYS)
      enforceSafeNettyVersion()
      enforceSafeProtobufVersion()
      enforceSafeJacksonToolsVersion()
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
