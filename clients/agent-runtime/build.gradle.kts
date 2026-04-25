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
  val homeCargoExecutable =
    providers
      .environmentVariable("HOME")
      .orNull
      ?.takeIf(String::isNotBlank)
      ?.let { File(it).resolve(".cargo/bin/cargo") }
      ?.takeIf { it.isFile && it.canExecute() }
      ?.absolutePath

  return configuredCargo?.takeIf(String::isNotBlank) ?: homeCargoExecutable ?: "cargo"
}

val cargoExecutable = resolveCargoExecutable()

fun registerCargoTask(name: String, taskDescription: String, vararg args: String) =
  tasks.register<Exec>(name) {
    group = "rust"
    description = taskDescription
    workingDir = isolated.projectDirectory.asFile
    commandLine(cargoExecutable, *args)
    enabled = isRustTasksEnabled
  }

val cargoCheck =
  registerCargoTask(
    name = "cargoCheck",
    taskDescription = "Run cargo check for embedded Corvus core.",
    "check",
  )

val cargoBuild =
  registerCargoTask(
    name = "cargoBuild",
    taskDescription = "Build embedded Corvus core with Cargo.",
    "build",
    "--release",
  )

val cargoTest =
  registerCargoTask(
    name = "cargoTest",
    taskDescription = "Run embedded Corvus test suite with Cargo.",
    "test",
    "--locked",
  )

val cargoFmtCheck =
  registerCargoTask(
    name = "cargoFmtCheck",
    taskDescription = "Verify Rust formatting for embedded Corvus core.",
    "fmt",
    "--all",
    "--",
    "--check",
  )

val cargoClippy =
  registerCargoTask(
    name = "cargoClippy",
    taskDescription = "Run cargo clippy for embedded Corvus core.",
    "clippy",
    "--all-targets",
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
