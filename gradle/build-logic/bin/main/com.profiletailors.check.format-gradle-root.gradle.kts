@file:Suppress("UnstableApiUsage")

import com.profiletailors.plugin.DefaultProjects
import com.profiletailors.plugin.spotless.SpotlessConfig
import com.profiletailors.plugin.spotless.SpotlessConfig.spotlessFileTree
import com.profiletailors.plugin.spotless.SpotlessLicense
import com.profiletailors.plugin.spotless.defaultStep
import com.profiletailors.plugin.spotless.nodeFile

plugins { id("com.profiletailors.check.format-base") }

// only root project
if (path == ":") {
  val buildLogic = DefaultProjects.buildLogic
  val subProjects =
    listOf(buildLogic, DefaultProjects.versions, DefaultProjects.aggregation, DefaultProjects.docs)

  spotless {
    val ktAndKtsFiles =
      spotlessFileTree("${buildLogic}/src").apply { include("**/*.kt", "**/*.gradle.kts") }

    val spotlessLicenseHeader = SpotlessLicense.getComment(project)

    kotlinGradle {
      defaultStep {
        ktfmt(SpotlessConfig.ktfmtVersion).googleStyle().configure {
          it.setRemoveUnusedImports(true)
        }
      }
      target(
        isolated.projectDirectory.files(
          "settings.gradle.kts",
          "build.gradle.kts",
          "${buildLogic}/settings.gradle.kts",
          subProjects.map { "${it}/build.gradle.kts" }.toTypedArray(),
        ),
        ktAndKtsFiles.matching { include("**/*.gradle.kts") },
      )
      if (spotlessLicenseHeader.isNotBlank()) {
        licenseHeader(spotlessLicenseHeader, "(^(?![\\/ ]\\*).*$)")
      }
    }
    kotlin {
      defaultStep {
        ktfmt(SpotlessConfig.ktfmtVersion).googleStyle().configure {
          it.setRemoveUnusedImports(true)
        }
      }
      target(ktAndKtsFiles.matching { include("**/*.kt") })
      if (spotlessLicenseHeader.isNotBlank()) {
        licenseHeader(spotlessLicenseHeader)
      }
    }

    val misc = listOf("**/*.md", "**/*.json", "**/*.json5", "**/*.yaml", "**/*.yml")
    val xml = listOf("**/*.xml")
    val targetFiles = spotlessFileTree("gradle/configs").apply { include(misc + xml) }
    val nodeExecutable = nodeFile().orNull

    format("prettierXmlRoot") {
      defaultStep {
        prettier(SpotlessConfig.prettierDevDependenciesWithXmlPlugin)
          .nodeExecutable(nodeExecutable)
          .config(
            mapOf(
              "plugins" to listOf("@prettier/plugin-xml"),
              "parser" to "xml",
              "useTabs" to false,
              "tabWidth" to 2,
            )
          )
      }
      target(targetFiles.matching { include(xml) })
    }
    format("prettierMiscRoot") {
      defaultStep {
        prettier(SpotlessConfig.prettierDevDependencies).nodeExecutable(nodeExecutable)
      }
      target(
        isolated.projectDirectory.files("README.md"),
        spotlessFileTree(".github").include(misc).exclude("**/*-lock.yaml"),
        targetFiles.matching {
          include(misc)
          exclude("**/*-lock.yaml")
        },
      )
    }
  }
}
