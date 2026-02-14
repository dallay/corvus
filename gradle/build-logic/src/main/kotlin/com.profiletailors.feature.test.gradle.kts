@file:Suppress("UnstableApiUsage")

import PluginHelpers.useJUnitJupiterM2
import com.profiletailors.plugin.environment.buildProperties
import com.profiletailors.plugin.local.LocalConfig
import com.profiletailors.plugin.local.getPropOrDefault
import java.nio.charset.StandardCharsets

plugins {
  java
  // https://docs.gradle.org/nightly/userguide/jacoco_plugin.html
  jacoco
}

val jepEnablePreview = project.getPropOrDefault(LocalConfig.Props.JEP_ENABLE_PREVIEW).toBoolean()

val buildProperties = project.buildProperties()
val junitJupiterM2Enabled =
  buildProperties
    .getProperty("junit.jupiter.m2.enabled", "")
    .ifBlank { project.getPropOrDefault(LocalConfig.Props.JUNIT_JUPITER_M2_ENABLED) }
    .toBoolean()

testing {
  suites {
    named<JvmTestSuite>("test") {
      targets.configureEach {
        // Use JUnit 5 as test framework
        useJUnitJupiter()

        // Configure details for test executions directly on 'Test' task
        testTask.configure {
          group = "verification"
          maxParallelForks = 4
          testLogging.showStandardStreams = true
          maxHeapSize = "1g"
          // Enable dynamic agent loading for tests - eg: Mockito, ByteBuddy
          jvmArgs(
            mutableListOf("-XX:+EnableDynamicAgentLoading").also {
              if (jepEnablePreview) {
                it.add("--enable-preview")
              }
            }
          )
          systemProperty("file.encoding", StandardCharsets.UTF_8.name())
        }
      }
    }
  }
}

if (junitJupiterM2Enabled) {
  useJUnitJupiterM2()
}

configurations.testCompileOnly { extendsFrom(configurations.compileOnly.get()) }

tasks.check { dependsOn(tasks.jacocoTestReport) }
