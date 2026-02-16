@file:Suppress("UnstableApiUsage")

import com.profiletailors.plugin.local.LocalConfig
import com.profiletailors.plugin.local.getPropOrDefault

plugins {
  id("com.profiletailors.module.kotlin")
  id("com.profiletailors.module.app")
  id("com.profiletailors.feature.test-end2end")
  id("com.profiletailors.feature.benchmark") apply false
}

val isJmh = project.getPropOrDefault(LocalConfig.Props.IS_JMH).toBoolean()

if (isJmh) {
  apply(plugin = "com.profiletailors.feature.benchmark")
}

application { mainClass.set("com.profiletailors.app.Application") }

dependencies {
  implementation("org.slf4j:slf4j-api")
  runtimeOnly("org.slf4j:slf4j-simple")
}

dependencies { mockApiImplementation(projects.app) }

dependencies {
  testEndToEndImplementation(projects.app) { capabilities { requireFeature("mock-api") } }
}
