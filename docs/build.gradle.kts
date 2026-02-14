@file:Suppress("UnstableApiUsage")

import com.github.gradle.node.pnpm.task.PnpmTask

plugins {
  alias(libs.plugins.com.github.node.gradle.node)
  id("com.profiletailors.check.format-base")
}

val nodeVersion = libs.versions.node.get()

val nodeDir = isolated.rootProject.projectDirectory.dir(".gradle/nodejs")

node {
  download.set(true)
  version.set(nodeVersion)
  distBaseUrl.set(null)
  workDir.set(nodeDir)
  nodeProjectDir.set(file("website"))
}

@Suppress("ConstPropertyName")
object StarlightConfig {
  const val starlightDist = "dist"
  const val distDir = starlightDist
}

"website"
  .also { website ->
    val starlightdoc =
      tasks.register<PnpmTask>("docStarlight") {
        group = "docs"
        description = "Generate Starlight docs"
        dependsOn(tasks.pnpmInstall)
        args.set(listOf("run", "build"))
      }

    val websiteFormat =
      tasks.register<PnpmTask>("websiteFormat") {
        group = "docs"
        description = "Format website sources with Biome"
        dependsOn(tasks.pnpmInstall)
        args.set(listOf("run", "format"))
      }

    val websiteCheck =
      tasks.register<PnpmTask>("websiteCheck") {
        group = "docs"
        description = "Check website sources with Biome"
        dependsOn(tasks.pnpmInstall)
        args.set(listOf("run", "check"))
      }

    tasks.register<Zip>("distZipWebsite") {
      group = "toolbox"
      description = "Zips the website dist directory"
      archiveFileName = "dist.zip"
      destinationDirectory.set(isolated.projectDirectory.dir("build/distributions"))
      from(isolated.projectDirectory.dir("${website}/${StarlightConfig.distDir}"))
      dependsOn(starlightdoc)
    }
    tasks.named("qualityCheck") { dependsOn(websiteCheck) }
    tasks.named("qualityGate") { dependsOn(websiteFormat) }
  }
