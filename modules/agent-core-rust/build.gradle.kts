@file:Suppress("UnstableApiUsage")

plugins {
  id("com.profiletailors.base.identity")
  id("com.profiletailors.check.format-gradle")
}

val isRustTasksEnabled =
  providers.gradleProperty("enableRustTasks").map(String::toBoolean).orElse(false).get()

fun registerCargoTask(name: String, description: String, vararg args: String) =
  tasks.register<Exec>(name) {
    group = "rust"
    this.description = description
    workingDir = isolated.projectDirectory.asFile
    commandLine("cargo", *args)
    enabled = isRustTasksEnabled
  }

val cargoCheck =
  registerCargoTask(
    name = "cargoCheck",
    description = "Run cargo check for embedded ZeroClaw core.",
    "check",
  )

val cargoBuild =
  registerCargoTask(
    name = "cargoBuild",
    description = "Build embedded ZeroClaw core with Cargo.",
    "build",
    "--release",
  )

val cargoTest =
  registerCargoTask(
    name = "cargoTest",
    description = "Run embedded ZeroClaw test suite with Cargo.",
    "test",
    "--locked",
  )

val cargoFmtCheck =
  registerCargoTask(
    name = "cargoFmtCheck",
    description = "Verify Rust formatting for embedded ZeroClaw core.",
    "fmt",
    "--all",
    "--",
    "--check",
  )

tasks.named("assemble") { dependsOn(cargoBuild) }

tasks.named("qualityCheck") {
  dependsOn(cargoCheck)
  dependsOn(cargoFmtCheck)
}

tasks.named("check") { dependsOn(cargoTest) }
