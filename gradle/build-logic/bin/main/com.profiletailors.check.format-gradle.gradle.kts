@file:Suppress("UnstableApiUsage")

import com.profiletailors.plugin.spotless.SortDependenciesStep
import com.profiletailors.plugin.spotless.SpotlessConfig
import com.profiletailors.plugin.spotless.SpotlessLicense
import com.profiletailors.plugin.spotless.defaultStep

plugins { id("com.profiletailors.check.format-base") }

spotless {
  kotlinGradle {
    defaultStep {
      addStep(SortDependenciesStep.create())
      ktfmt(SpotlessConfig.ktfmtVersion).googleStyle().configure { it.setRemoveUnusedImports(true) }
    }
    target(
      isolated.projectDirectory.files("settings.gradle.kts", "build.gradle.kts", "build.gradle.kt")
    )
    val spotlessLicenseHeader = SpotlessLicense.getComment(project)
    if (spotlessLicenseHeader.isNotBlank()) {
      licenseHeader(spotlessLicenseHeader, "(^(?![\\/ ]\\*).*$)")
    }
  }
}
