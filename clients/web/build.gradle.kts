@file:Suppress("UnstableApiUsage")

plugins {
  id("com.profiletailors.check.format-base")
}

val pnpmShim = isolated.rootProject.projectDirectory.file("gradle/bin/pnpm").asFile.absolutePath
val webRootDir = isolated.projectDirectory.asFile
val appsDir = file("${webRootDir}/apps")

// Discover all web apps dynamically
val webApps = appsDir.listFiles { file ->
  file.isDirectory && file.resolve("package.json").exists()
}?.map { it.name } ?: emptyList()

logger.lifecycle("📦 Web apps discovered: ${webApps.joinToString()}")

// Configuration for each app type
data class WebAppConfig(
  val name: String,
  val distDir: String = "dist",
  val port: Int = 4321,
)

// App configurations (extend as needed)
val appConfigs = mapOf(
  "docs" to WebAppConfig("docs", "dist", 4321),
  "landing" to WebAppConfig("landing", "dist", 4322),
  "dashboard" to WebAppConfig("dashboard", "dist", 4323),
)

// Root workspace install task
val workspaceInstall =
  tasks.register<Exec>("workspaceInstall") {
    group = "web"
    description = "Install all workspace dependencies with pnpm"
    workingDir = webRootDir
    commandLine(pnpmShim, "install", "--frozen-lockfile")
    inputs.files("package.json", "pnpm-lock.yaml", "pnpm-workspace.yaml")
    outputs.dir("node_modules")
  }

// Generate tasks for each web app
webApps.forEach { appName ->
  val appDir = file("${appsDir}/${appName}")
  val config = appConfigs[appName] ?: WebAppConfig(appName)

  // Install task (depends on workspace install)
  val appInstall =
    tasks.register<Exec>("${appName}Install") {
      group = "web"
      description = "Install dependencies for ${appName}"
      dependsOn(workspaceInstall)
      workingDir = appDir
      commandLine(pnpmShim, "install", "--frozen-lockfile")
      inputs.file("${appDir}/package.json")
      outputs.dir("${appDir}/node_modules")
    }

  // Build task
  val appBuild =
    tasks.register<Exec>("${appName}Build") {
      group = "web"
      description = "Build ${appName} app"
      dependsOn(appInstall)
      workingDir = appDir
      commandLine(pnpmShim, "run", "build")
      inputs.dir(appDir)
      outputs.dir("${appDir}/${config.distDir}")
    }

  // Dev task (for local development)
  tasks.register<Exec>("${appName}Dev") {
    group = "web"
    description = "Start ${appName} dev server on port ${config.port}"
    dependsOn(appInstall)
    workingDir = appDir
    commandLine(pnpmShim, "run", "dev")
  }

  // Format task
  val appFormat =
    tasks.register<Exec>("${appName}Format") {
      group = "web"
      description = "Format ${appName} sources"
      dependsOn(appInstall)
      workingDir = appDir
      commandLine(pnpmShim, "run", "format")
      isIgnoreExitValue = true // Some apps may not have format script yet
    }

  // Check task
  val appCheck =
    tasks.register<Exec>("${appName}Check") {
      group = "web"
      description = "Check ${appName} sources"
      dependsOn(appInstall)
      workingDir = appDir
      commandLine(pnpmShim, "run", "check")
      isIgnoreExitValue = true // Some apps may not have check script yet
    }

  // Distribution zip
  tasks.register<Zip>("${appName}DistZip") {
    group = "web"
    description = "Zip ${appName} distribution"
    archiveFileName = "${appName}-dist.zip"
    destinationDirectory.set(file("${webRootDir}/build/distributions"))
    from("${appDir}/${config.distDir}")
    dependsOn(appBuild)
  }

  // Clean task
  tasks.register<Delete>("${appName}Clean") {
    group = "web"
    description = "Clean ${appName} build artifacts"
    delete("${appDir}/${config.distDir}")
    delete("${appDir}/node_modules")
  }

  // Hook into quality checks
  tasks.named("qualityCheck") { dependsOn(appCheck) }
  tasks.named("qualityGate") { dependsOn(appFormat) }
}

// Aggregate tasks for all apps
tasks.register("buildAllWebApps") {
  group = "web"
  description = "Build all web applications"
  dependsOn(webApps.map { "${it}Build" })
}

tasks.register("cleanAllWebApps") {
  group = "web"
  description = "Clean all web applications"
  dependsOn(webApps.map { "${it}Clean" })
}

tasks.register("distZipAllWebApps") {
  group = "web"
  description = "Create distribution zips for all web apps"
  dependsOn(webApps.map { "${it}DistZip" })
}

// Legacy compatibility with old "docs" naming
tasks.register<Exec>("websiteInstall") {
  group = "web"
  description = "[Legacy] Install docs dependencies"
  dependsOn(workspaceInstall)
}

tasks.register<Exec>("docStarlight") {
  group = "web"
  description = "[Legacy] Build Starlight docs"
  dependsOn("docsBuild")
}

tasks.register<Exec>("websiteFormat") {
  group = "web"
  description = "[Legacy] Format docs"
  dependsOn("docsFormat")
}

tasks.register<Exec>("websiteCheck") {
  group = "web"
  description = "[Legacy] Check docs"
  dependsOn("docsCheck")
}

tasks.register<Zip>("distZipWebsite") {
  group = "web"
  description = "[Legacy] Zip docs distribution"
  dependsOn("docsDistZip")
}
