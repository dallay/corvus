plugins {
  id("com.profiletailors.base.identity")
  id("com.profiletailors.base.lifecycle")
  id("com.profiletailors.feature.aggregation")
  id("com.profiletailors.check.format-gradle")
  id("com.profiletailors.report.sbom")
  id("com.profiletailors.report.test")
  id("com.profiletailors.report.code-coverage")
}

dependencies { runCatching { implementation(project(":app")) } }
