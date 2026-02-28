@file:Suppress("UnstableApiUsage")

plugins {
  id("com.profiletailors.base.identity")
  id("com.profiletailors.check.format-gradle")
  id("com.profiletailors.check.format-kotlin")
  id("org.jetbrains.kotlinx.kover")
  alias(libs.plugins.org.jetbrains.kotlin.multiplatform)
}

kotlin {
  jvmToolchain(libs.versions.jdk.get().toInt())

  jvm()

  sourceSets {
    val commonMain by getting

    val commonTest by getting { dependencies { implementation(kotlin("test")) } }

    val jvmMain by getting
    val jvmTest by getting
  }
}
