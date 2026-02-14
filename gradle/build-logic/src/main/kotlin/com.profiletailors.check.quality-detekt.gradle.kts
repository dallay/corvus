@file:Suppress("UnstableApiUsage", "detekt:SpreadOperator")

import dev.detekt.gradle.Detekt
import dev.detekt.gradle.plugin.getSupportedKotlinVersion

plugins {
  // https://github.com/detekt/detekt
  id("dev.detekt")
  id("com.profiletailors.base.lifecycle")
}

val gradleKotlinVersion = embeddedKotlinVersion
val detektKotlinVersion = getSupportedKotlinVersion()
val enableDetekt = detektKotlinVersion >= gradleKotlinVersion

tasks.named("qualityCheck") { dependsOn(tasks.detekt) }

tasks.named("qualityGate") { dependsOn(tasks.detekt) }

// default excludes.
val defaultDetektExcludes = arrayOf("**/nocheck/**", "**/autogen/**", "**/generated/**")

tasks.withType<Detekt>().configureEach {
  enabled = true
  exclude(*defaultDetektExcludes)
}

val detektYml =
  layout.projectDirectory.file("configs/detekt/detekt.yml").asFile.takeIf { it.exists() }
    ?: isolated.rootProject.projectDirectory
      .file("gradle/configs/detekt/detekt.yml")
      .asFile
      .takeIf { it.exists() }

detekt {
  debug = false
  if (detektYml != null) {
    config.setFrom(detektYml)
    buildUponDefaultConfig = true
  }
}
