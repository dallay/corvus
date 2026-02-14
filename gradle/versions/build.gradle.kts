plugins {
  id("com.profiletailors.base.identity")
  id("com.profiletailors.base.lifecycle")
  id("com.profiletailors.base.jvm-conflict")
  id("com.profiletailors.feature.use-all-catalog-versions")
  id("com.profiletailors.check.format-gradle")
  id("com.profiletailors.tools.check-version")
}

javaPlatform.allowDependencies()

dependencies { api(platform(libs.slf4jBom)) }

// Reject versions that should not be upgraded beyond a certain point.
// This makes Dependabot PR builds fail that attempt to update these.
// https://docs.gradle.org/nightly/userguide/dependency_constraints.html
dependencies.constraints {}
