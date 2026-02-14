import com.profiletailors.plugin.spotless.SpotlessConfig
import com.profiletailors.plugin.spotless.SpotlessConfig.spotlessFileTree
import com.profiletailors.plugin.spotless.SpotlessLicense
import com.profiletailors.plugin.spotless.defaultStep
import com.profiletailors.plugin.spotless.nodeFile

plugins { id("com.profiletailors.check.format-base") }

spotless {
  shell { defaultStep { shfmt() } }

  val misc = listOf("**/*.md", "**/*.json", "**/*.json5", "**/*.yaml", "**/*.yml")
  val xml = listOf("**/*.xml")
  val targetFiles = spotlessFileTree().apply { include(misc + xml) }

  format("prettierXml") {
    defaultStep {
      prettier(SpotlessConfig.prettierDevDependenciesWithXmlPlugin)
        .nodeExecutable(nodeFile().orNull)
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
    val spotlessLicenseHeader = SpotlessLicense.getXml(project)
    if (spotlessLicenseHeader.isNotBlank()) {
      licenseHeader(spotlessLicenseHeader, "(<[^!?])")
    }
  }
  format("prettierMisc") {
    defaultStep {
      prettier(SpotlessConfig.prettierDevDependencies).nodeExecutable(nodeFile().orNull)
    }
    target(
      targetFiles.matching {
        include(misc)
        exclude("**/*-lock.yaml")
      }
    )
  }
}
