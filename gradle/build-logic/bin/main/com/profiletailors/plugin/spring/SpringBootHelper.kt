package com.profiletailors.plugin.spring

import org.gradle.api.Project
import org.gradle.kotlin.dsl.dependencies
import org.springframework.boot.gradle.tasks.bundling.BootBuildImage
import org.springframework.boot.gradle.util.VersionExtractor

/** A helper class for Spring Boot projects. */
object SpringBootHelper {

  /**
   * Returns the version of the Spring Boot platform.
   *
   * @return The version of the Spring Boot platform.
   */
  fun getSpringBootVersion(default: String = "4.0.0"): String {
    return VersionExtractor.forClass(BootBuildImage::class.java) ?: default
  }

  /**
   * Adds a dependency on the Spring Boot auto module.
   *
   * @param autoKspVersion The version of the dependency.
   * @param springBootVersion The version of the dependency.
   */
  fun Project.kspSpringBootAuto(autoKspVersion: String = "1.0.3", springBootVersion: String = "") {
    val bootVersion = if (springBootVersion.isNotBlank()) ":$springBootVersion" else ""
    dependencies {
      add("ksp", "com.profiletailors:mica-auto-ksp:$autoKspVersion")
      add("implementation", "com.profiletailors:mica-auto-ksp:$autoKspVersion")
      add("implementation", "org.springframework.boot:spring-boot-autoconfigure$bootVersion")
    }
  }

  /**
   * Adds a dependency on the Spring Boot processor.
   *
   * @param springBootVersion The version of the dependency.
   */
  fun Project.kaptSpringBootProcessor(springBootVersion: String = getSpringBootVersion()) {
    val bootVersion = if (springBootVersion.isNotBlank()) ":$springBootVersion" else ""
    dependencies {
      add("kapt", "org.springframework.boot:spring-boot-autoconfigure-processor$bootVersion")
      add("kapt", "org.springframework.boot:spring-boot-configuration-processor$bootVersion")
    }
  }

  /**
   * Adds a dependency on the Spring Boot starter web mvc.
   *
   * @param springBootVersion The version of the dependency.
   */
  fun Project.springBootStarterWebMvc(springBootVersion: String = "") {
    val bootVersion = if (springBootVersion.isNotBlank()) ":$springBootVersion" else ""
    dependencies {
      add("implementation", "org.springframework.boot:spring-boot-starter-webmvc$bootVersion")
      add(
        "testImplementation",
        "org.springframework.boot:spring-boot-starter-webmvc-test$bootVersion",
      )
    }
  }
}
