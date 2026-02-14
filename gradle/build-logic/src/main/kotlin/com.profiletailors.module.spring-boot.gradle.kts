@file:Suppress("UnstableApiUsage")

import com.profiletailors.plugin.tasks.GenerateStartScript

plugins {
  java
  id("com.profiletailors.module.app")
  // https://github.com/spring-projects/spring-boot/blob/main/build-plugin/spring-boot-gradle-plugin/src/docs/antora/modules/gradle-plugin/pages/index.adoc
  id("org.springframework.boot")
}

tasks.run { enabled = false }

configurations {
  val internal = maybeCreate("internal")
  // https://docs.spring.io/spring-boot/gradle-plugin/reacting.html
  // productionRuntimeClasspath = runtimeClasspath - developmentOnly/testDevelopmentOnly
  productionRuntimeClasspath {
    extendsFrom(internal)
    shouldResolveConsistentlyWith(configurations.named("runtimeClasspath").get())
  }
  developmentOnly {
    extendsFrom(internal)
    shouldResolveConsistentlyWith(configurations.named("runtimeClasspath").get())
  }
}

springBoot {
  buildInfo {
    properties {
      additional.put("project.path", project.path)
      additional.put(
        "layout.settingsDirectory",
        layout.settingsDirectory.asFile.invariantSeparatorsPath,
      )
      additional.put(
        "layout.projectDirectory",
        layout.projectDirectory.asFile.invariantSeparatorsPath,
      )
      additional.put(
        "layout.buildDirectory",
        layout.buildDirectory.get().asFile.invariantSeparatorsPath,
      )
    }
  }
}

tasks.named<GenerateStartScript>("generateStartScript") {
  appJar.set(tasks.bootJar.flatMap { it.archiveFileName })
}

tasks.named<Copy>("copyJarToRoot") { from(tasks.bootJar) }
