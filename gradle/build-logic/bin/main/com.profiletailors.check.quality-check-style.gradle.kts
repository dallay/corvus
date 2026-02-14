@file:Suppress("UnstableApiUsage")

import com.profiletailors.plugin.gradle.eagerDiskCache

plugins {
  // https://docs.gradle.org/current/userguide/checkstyle_plugin.html#checkstyle_plugin
  checkstyle
  id("com.profiletailors.base.lifecycle")
}

// auto bind to checks task:
// https://docs.gradle.org/current/userguide/checkstyle_plugin.html#dependencies_added_to_other_tasks
afterEvaluate {
  tasks.named("qualityCheck") {
    dependsOn(tasks.named("checkstyleMain"), tasks.named("checkstyleTest"))
  }
}

val eagerDiskCacheKey = "com.profiletailors.check.quality-check-style"

val projectRoot: Directory = isolated.rootProject.projectDirectory
val projectRootPath = projectRoot.asFile.invariantSeparatorsPath

val checkStyleRootPath = "${projectRootPath}/gradle/configs/checkstyle"
val checkStyleConfigFile: File = projectRoot.file("${checkStyleRootPath}/checkstyle.xml").asFile
val checkStyleConfigProperties =
  mapOf(
    "checkstyle.header.file" to "${checkStyleRootPath}/checkstyle-header-file.txt",
    "checkstyle.suppressions" to
      run {
        project.eagerDiskCache("$eagerDiskCacheKey.checkstyle.suppressions") {
          val file =
            isolated.projectDirectory.file("configs/checkstyle/checkstyle-suppressions.xml").asFile
          if (file.exists()) {
            file.invariantSeparatorsPath
          } else "${checkStyleRootPath}/checkstyle-suppressions.xml"
        }
      },
    "checkstyle.import.control" to
      run {
        project.eagerDiskCache("$eagerDiskCacheKey.checkstyle.import.control") {
          val file =
            isolated.projectDirectory
              .file("configs/checkstyle/checkstyle-import-control.xml")
              .asFile
          if (file.exists()) {
            file.invariantSeparatorsPath
          } else "${checkStyleRootPath}/checkstyle-import-control.xml"
        }
      },
  )

configure<CheckstyleExtension> {
  isShowViolations = true
  isIgnoreFailures = false
  maxWarnings = 0
  configFile = checkStyleConfigFile
  configProperties = checkStyleConfigProperties
  // https://docs.gradle.org/nightly/dsl/org.gradle.api.plugins.quality.CheckstyleExtension.html#org.gradle.api.plugins.quality.CheckstyleExtension:reportsDir
  //  reportsDir = reporting.baseDirectory.dir("checkstyle").get().asFile
}
