import com.profiletailors.plugin.includeProjects

pluginManagement { includeBuild("gradle/build-logic") }

plugins {
  id("com.profiletailors.build")
  id("com.profiletailors.build.feature.catalogs")
  id("com.profiletailors.plugin.example.settings")
  id("org.gradle.toolchains.foojay-resolver-convention") version "1.0.0"
}

rootProject.name = "corvus"

includeProjects(
  mapOf(
    ":docs" to "apps/docs",
    ":composeApp" to "apps/composeApp",
    ":agent-core-rust" to "modules/agent-core-rust",
    ":agent-core-kmp" to "modules/agent-core-kmp",
  )
)
