@file:Suppress("UnstableApiUsage")

plugins {
  id("com.profiletailors.check.format-base")
}

val pnpmShim = isolated.rootProject.projectDirectory.file("gradle/bin/pnpm").asFile.absolutePath
val websiteDir = isolated.projectDirectory.dir("website").asFile

@Suppress("ConstPropertyName")
object StarlightConfig {
  const val starlightDist = "dist"
  const val distDir = starlightDist
}

"website"
  .also { website ->
    val websiteInstall =
      tasks.register<Exec>("websiteInstall") {
        group = "docs"
        description = "Install website dependencies with pnpm"
        workingDir = websiteDir
        commandLine(pnpmShim, "install", "--frozen-lockfile")
      }

    val starlightdoc =
      tasks.register<Exec>("docStarlight") {
        group = "docs"
        description = "Generate Starlight docs"
        dependsOn(websiteInstall)
        workingDir = websiteDir
        commandLine(pnpmShim, "run", "build")
      }

    val websiteFormat =
      tasks.register<Exec>("websiteFormat") {
        group = "docs"
        description = "Format website sources with Biome"
        dependsOn(websiteInstall)
        workingDir = websiteDir
        commandLine(pnpmShim, "run", "format")
      }

    val websiteCheck =
      tasks.register<Exec>("websiteCheck") {
        group = "docs"
        description = "Check website sources with Biome"
        dependsOn(websiteInstall)
        workingDir = websiteDir
        commandLine(pnpmShim, "run", "check")
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
