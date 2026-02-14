@file:Suppress("PackageDirectoryMismatch")

import com.profiletailors.plugin.injected
import org.gradle.api.tasks.PathSensitivity

plugins { base }

/**
 * AgentSync Gradle Plugin
 *
 * Keep AI assistant configurations synchronized across different environments. See:
 * https://dallay.github.io/agentsync/
 */
val agentsyncApply =
  tasks.register("agentsyncApply") {
    group = "toolbox"
    description = "Synchronizes AI agent configurations using AgentSync"

    val inject = injected
    val agentsDir = rootProject.file(".agents")

    // Register inputs for incremental build support
    inputs
      .dir(agentsDir)
      .optional()
      .withPropertyName("agentsDir")
      .withPathSensitivity(PathSensitivity.RELATIVE)

    doLast {
      if (!agentsDir.exists()) {
        logger.info("Skipping agentsyncApply: .agents directory not found")
        return@doLast
      }

      val commands =
        listOf(
          listOf("agentsync", "apply"),
          listOf("npx", "@dallay/agentsync", "apply"),
          listOf("pnpm", "dlx", "@dallay/agentsync", "apply"),
        )

      val successfulCommand =
        commands.firstOrNull { command ->
          val exitCode =
            try {
              inject.exec
                .exec {
                  commandLine(command)
                  isIgnoreExitValue = true
                }
                .exitValue
            } catch (_: Exception) {
              logger.info("AgentSync command unavailable: ${command.joinToString(" ")}")
              Int.MIN_VALUE
            }

          val succeeded = exitCode == 0
          if (!succeeded && exitCode != Int.MIN_VALUE) {
            logger.info(
              "AgentSync command failed (${command.joinToString(" ")}), exit code: $exitCode"
            )
          }
          succeeded
        }

      if (successfulCommand != null) {
        println("✓ AI agents synchronized successfully (${successfulCommand.joinToString(" ")})")
      } else {
        // We print a message instead of failing the build to make it optional
        println(
          "⚠️ AgentSync no está disponible. Intenté: agentsync, npx y pnpm dlx. " +
            "Visita https://dallay.github.io/agentsync/ para instalarlo."
        )
      }
    }
  }

// Run AgentSync as part of the standard verification lifecycle
tasks.named("check") { dependsOn(agentsyncApply) }
