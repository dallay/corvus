@file:Suppress("UnstableApiUsage")

import java.io.File

plugins { id("com.autonomousapps.dependency-analysis") }

fun findGradleWrapper(startDir: File): File? {
  val wrapperName =
    if (org.gradle.internal.os.OperatingSystem.current().isWindows) "gradlew.bat" else "gradlew"

  return generateSequence(startDir) { it.parentFile }
    .map { it.resolve(wrapperName) }
    .firstOrNull { it.isFile }
}

if (path == ":") {
  dependencyAnalysis {
    // https://github.com/autonomousapps/dependency-analysis-gradle-plugin/issues/1234
    // https://github.com/autonomousapps/dependency-analysis-gradle-plugin/issues/1485
    structure {
      bundle("spring-boot-starter-webmvc") {
        primary("org.springframework.boot:spring-boot-starter-webmvc")
        includeDependency("org.springframework.boot:spring-boot")
        includeDependency("org.springframework.boot:spring-boot-autoconfigure")
        includeDependency("org.springframework:spring-web")
        includeDependency("org.springframework:spring-context")
        includeDependency("org.springframework:spring-aop")
        includeDependency("org.springframework:spring-beans")
        includeDependency("org.springframework:spring-expression")
        includeDependency("org.springframework:spring-core")
      }
      bundle("spring-boot-starter-webmvc-test") {
        primary("org.springframework.boot:spring-boot-starter-webmvc-test")
        includeDependency("org.springframework.boot:spring-boot-webmvc-test")
        includeDependency("org.springframework.boot:spring-boot-resttestclient")
        includeDependency("org.springframework.boot:spring-boot-test")
        includeDependency("org.springframework:spring-test")
        includeDependency("org.junit.jupiter:junit-jupiter-api")
        includeDependency("org.assertj:assertj-core")
      }
    }
  }

  val gradleWrapper = provider {
    findGradleWrapper(rootDir)
      ?: error("Could not locate Gradle wrapper from ${rootDir.absolutePath}")
  }

  val writeLocksBuildLogic =
    tasks.register<Exec>("writeLocksBuildLogic") {
      group = "toolbox"
      description = "Write dependency lockfiles for gradle/build-logic."
      notCompatibleWithConfigurationCache("Runs nested Gradle commands for the included build.")
      workingDir = rootDir
      commandLine(gradleWrapper.get().absolutePath, "-p", "gradle/build-logic", "writeLocks")
    }

  val checkLocksBuildLogic =
    tasks.register<Exec>("checkLocksBuildLogic") {
      group = "toolbox"
      description = "Verify dependency lockfiles for gradle/build-logic."
      notCompatibleWithConfigurationCache("Runs nested Gradle commands for the included build.")
      workingDir = rootDir
      commandLine(gradleWrapper.get().absolutePath, "-p", "gradle/build-logic", "checkLocks")
    }

  val mainBuildWriteLockTasks = provider {
    allprojects.map { currentProject ->
      if (currentProject.path == ":") "writeLocks" else "${currentProject.path}:writeLocks"
    }
  }

  val mainBuildCheckLockTasks = provider {
    allprojects.map { currentProject ->
      if (currentProject.path == ":") "checkLocks" else "${currentProject.path}:checkLocks"
    }
  }

  tasks.register("writeLocksAll") {
    group = "toolbox"
    description = "Write dependency lockfiles for every Gradle project and included build."
    dependsOn(mainBuildWriteLockTasks)
    dependsOn(writeLocksBuildLogic)
  }

  tasks.register("checkLocksAll") {
    group = "toolbox"
    description = "Verify dependency lockfiles for every Gradle project and included build."
    dependsOn(mainBuildCheckLockTasks)
    dependsOn(checkLocksBuildLogic)
  }
}
