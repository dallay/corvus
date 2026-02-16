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
    ":androidApp" to "clients/androidApp",
    ":web" to "clients/web",
    ":composeApp" to "clients/composeApp",
    ":agent-runtime" to "clients/agent-runtime",
    ":agent-core-kmp" to "modules/agent-core-kmp",
  )
)
