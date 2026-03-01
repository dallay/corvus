@file:Suppress("UnstableApiUsage")

import java.io.File

plugins {
  id("com.profiletailors.base.identity")
  id("com.profiletailors.check.format-gradle")
}

val isRustTasksEnabled =
  providers.gradleProperty("enableRustTasks").map(String::toBoolean).orElse(false).get()

fun resolveCargoExecutable(): String {
  val configuredCargo = providers.environmentVariable("CARGO").orNull
  if (!configuredCargo.isNullOrBlank()) {
    return configuredCargo
  }

  val homeDir = providers.environmentVariable("HOME").orNull
  if (!homeDir.isNullOrBlank()) {
    val homeCargo = File(homeDir).resolve(".cargo/bin/cargo")
    if (homeCargo.isFile && homeCargo.canExecute()) {
      return homeCargo.absolutePath
    }
  }

  return "cargo"
}

val cargoExecutable = resolveCargoExecutable()

fun registerCargoTask(name: String, description: String, vararg args: String) =
  tasks.register<Exec>(name) {
    group = "rust"
    this.description = description
    workingDir = isolated.projectDirectory.asFile
    commandLine(cargoExecutable, *args)
    enabled = isRustTasksEnabled
  }

val cargoCheck =
  registerCargoTask(
    name = "cargoCheck",
    description = "Run cargo check for embedded Corvus core.",
    "check",
  )

val cargoBuild =
  registerCargoTask(
    name = "cargoBuild",
    description = "Build embedded Corvus core with Cargo.",
    "build",
    "--release",
  )

val cargoTest =
  registerCargoTask(
    name = "cargoTest",
    description = "Run embedded Corvus test suite with Cargo.",
    "test",
    "--locked",
  )

val cargoFmtCheck =
  registerCargoTask(
    name = "cargoFmtCheck",
    description = "Verify Rust formatting for embedded Corvus core.",
    "fmt",
    "--all",
    "--",
    "--check",
  )

val cargoClippy =
  registerCargoTask(
    name = "cargoClippy",
    description = "Run cargo clippy for embedded Corvus core.",
    "clippy",
    "--all-targets",
    "--all-features",
    "--",
    "-D",
    "warnings",
  )

tasks.named("assemble") { dependsOn(cargoBuild) }

tasks.named("qualityCheck") {
  dependsOn(cargoCheck)
  dependsOn(cargoFmtCheck)
  dependsOn(cargoClippy)
}

tasks.named("check") {
  dependsOn(cargoTest)
  dependsOn(cargoClippy)
}
