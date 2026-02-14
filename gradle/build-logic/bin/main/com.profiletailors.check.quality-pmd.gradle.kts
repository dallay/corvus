@file:Suppress("UnstableApiUsage")

plugins {
  // https://docs.gradle.org/current/userguide/pmd_plugin.html#pmd_plugin
  pmd
  id("com.profiletailors.base.lifecycle")
}

// auto bind to checks task:
// https://docs.gradle.org/current/userguide/pmd_plugin.html#sec:pmd_tasks
afterEvaluate {
  tasks.named("qualityCheck") { dependsOn(tasks.named("pmdMain"), tasks.named("pmdTest")) }
}

dependencies {
  // p3c rule is no longer maintained, https://github.com/godfather1103/p3c
  //  pmd("io.github.godfather1103.p3c:p3c-pmd:2.1.1-ext-6")
}

val pmdConfigFile =
  layout.projectDirectory.files("configs/pmd/pmd.xml").takeIf { it.files.isNotEmpty() }
    ?: isolated.rootProject.projectDirectory.files("gradle/configs/pmd/pmd.xml").takeIf {
      it.files.isNotEmpty()
    }

tasks {
  pmd {
    threads = 4
    isConsoleOutput = false
    isIgnoreFailures = false
    incrementalAnalysis = true
    ruleSets = emptyList()
    if (pmdConfigFile != null) {
      ruleSetFiles = pmdConfigFile
    }
    // https://docs.gradle.org/current/dsl/org.gradle.api.plugins.quality.PmdExtension.html#org.gradle.api.plugins.quality.PmdExtension:reportsDir
    //  reportsDir = reporting.baseDirectory.dir("pmd").get().asFile
  }
}
