import com.profiletailors.plugin.includeProjects

pluginManagement { includeBuild("gradle/build-logic") }

plugins {
  id("com.profiletailors.build")
  id("com.profiletailors.build.feature.catalogs")
  id("com.profiletailors.plugin.example.settings")
  id("org.gradle.toolchains.foojay-resolver-convention") version "1.0.0"
}

rootProject.name = "profiletailors-gradle-plugin"

includeProjects(
  mapOf(
    ":docs" to "docs",
    ":app" to "app",
    ":example-java" to "examples/example-java",
    ":example-kotlin" to "examples/example-kotlin",
    ":example-spring" to "examples/example-spring",
  )
)
