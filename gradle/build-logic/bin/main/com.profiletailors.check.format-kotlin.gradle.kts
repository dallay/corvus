import com.profiletailors.plugin.spotless.SpotlessConfig
import com.profiletailors.plugin.spotless.SpotlessConfig.spotlessFileTree
import com.profiletailors.plugin.spotless.SpotlessLicense
import com.profiletailors.plugin.spotless.defaultStep
import com.profiletailors.plugin.tasks.FileContentCheck

plugins { id("com.profiletailors.check.format-base") }

val sources: FileTree = spotlessFileTree().matching { include("**/*.kt") }

spotless {
  kotlin {
    defaultStep {
      ktfmt(SpotlessConfig.ktfmtVersion).googleStyle().configure { it.setRemoveUnusedImports(true) }
    }
    target(sources)
    val spotlessLicenseHeader = SpotlessLicense.getComment(project)
    if (spotlessLicenseHeader.isNotBlank()) {
      licenseHeader(spotlessLicenseHeader)
    }
  }

  format("kotlinForbid") {
    SpotlessConfig.getForbidRegexList(project, "gradle/configs/spotless/forbid-regex.txt").forEach {
      addStep(it)
    }
    target(sources)
  }
}

tasks.named<FileContentCheck>("fileContentCheck") { sourceFiles = sources }
