@file:Suppress("UnstableApiUsage")

import java.io.File
import java.nio.charset.StandardCharsets
import java.util.concurrent.TimeUnit
import org.gradle.api.file.RegularFile
import org.gradle.api.provider.Provider

val isCi = providers.environmentVariable("CI").orNull?.isNotBlank() == true
val safeNettyVersion = "4.1.132.Final"
val safeProtobufVersion = "3.25.9"
val safeJacksonToolsVersion = "3.1.1"
val dynamicVersionCacheDurationDays = 7
val safeCommonsCompressVersion = "1.26.0"
val safeJose4jVersion = "0.9.6"
val safeBouncyCastleVersion = "1.84"
val safeJdom2Version = "2.0.6.1"

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

fun ResolutionStrategy.enforceSafeCommonsCompressVersion() {
  eachDependency {
    if (
      requested.group == "org.apache.commons" &&
        requested.name == "commons-compress" &&
        requested.version != safeCommonsCompressVersion
    ) {
      useVersion(safeCommonsCompressVersion)
      because("Commons Compress $safeCommonsCompressVersion addresses Dependabot vulnerabilities")
    }
  }
}

fun ResolutionStrategy.enforceSafeJose4jVersion() {
  eachDependency {
    if (
      requested.group == "org.bitbucket.b_c" &&
        requested.name == "jose4j" &&
        requested.version != safeJose4jVersion
    ) {
      useVersion(safeJose4jVersion)
      because(
        "jose4j $safeJose4jVersion fixes the JWE decompression denial of service vulnerability"
      )
    }
  }
}

fun ResolutionStrategy.enforceSafeBouncyCastleVersion() {
  eachDependency {
    if (requested.group == "org.bouncycastle" && requested.version != safeBouncyCastleVersion) {
      useVersion(safeBouncyCastleVersion)
      because(
        "Bouncy Castle $safeBouncyCastleVersion fixes the vulnerable OpenPGP parsing dependencies"
      )
    }
  }
}

fun ResolutionStrategy.enforceSafeJdom2Version() {
  eachDependency {
    if (
      requested.group == "org.jdom" &&
        requested.name == "jdom2" &&
        requested.version != safeJdom2Version
    ) {
      useVersion(safeJdom2Version)
      because(
        "JDOM $safeJdom2Version fixes the XXE-related parser hardening issue reported by Dependabot"
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
      cacheDynamicVersionsFor(dynamicVersionCacheDurationDays, TimeUnit.DAYS)
      enforceSafeNettyVersion()
      enforceSafeProtobufVersion()
      enforceSafeJacksonToolsVersion()
      enforceSafeCommonsCompressVersion()
      enforceSafeJose4jVersion()
      enforceSafeBouncyCastleVersion()
      enforceSafeJdom2Version()
      activateDependencyLocking()
    }
  }
}

configurations.configureEach {
  if (shouldUseDependencyLocking()) {
    resolutionStrategy {
      cacheDynamicVersionsFor(dynamicVersionCacheDurationDays, TimeUnit.DAYS)
      enforceSafeNettyVersion()
      enforceSafeProtobufVersion()
      enforceSafeJacksonToolsVersion()
      enforceSafeCommonsCompressVersion()
      enforceSafeJose4jVersion()
      enforceSafeBouncyCastleVersion()
      enforceSafeJdom2Version()
      activateDependencyLocking()
    }
  }
}

val lockFilesProvider: Provider<List<RegularFile>> = provider {
  listOf(
    layout.projectDirectory.file("buildscript-gradle.lockfile"),
    layout.projectDirectory.file("gradle.lockfile"),
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
      val lockFiles = lockFilesProvider.get()
      val backupFiles =
        lockFiles.associateWith { lockFile ->
          layout.buildDirectory.file("tmp/locks/${lockFile.asFile.name}.bak").get().asFile
        }

      backupFiles.forEach { (lockFile, backup) ->
        val file = lockFile.asFile
        if (file.exists()) {
          backup.parentFile.mkdirs()
          file.copyTo(backup, overwrite = true)
        }
      }
    }

    doLast {
      val isUnix = org.gradle.internal.os.OperatingSystem.current().isUnix
      if (!isUnix) {
        lockFilesProvider.get().forEach { lockFile ->
          val file = lockFile.asFile
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
    val lockFiles = lockFilesProvider.get()
    val backupFiles =
      lockFiles.associateWith { lockFile ->
        layout.buildDirectory.file("tmp/locks/${lockFile.asFile.name}.bak").get().asFile
      }

    lockFiles.forEach { lockFile ->
      val file = lockFile.asFile
      val backup = backupFiles.getValue(lockFile)
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
